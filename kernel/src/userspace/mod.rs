use x86_64::{
    structures::paging::{PageTableFlags, PageTable},
    VirtAddr,
};

use crate::process::{self, ProcessId, USER_CODE_BASE, USER_STACK_TOP, USER_STACK_SIZE};
use crate::mm::vmm;
use crate::scheduler::{self, task::PriorityClass};
use crate::debug::log::Logger;
use crate::loader::{self, LoadedBinary};

#[unsafe(naked)]
#[unsafe(no_mangle)]
#[link_section = ".user_text"]
pub unsafe extern "C" fn user_bootstrap() {
    core::arch::naked_asm!(
        "sub rsp, 16",
        "mov byte ptr [rsp+0], 0x55", // U
        "mov byte ptr [rsp+1], 0x53", // S
        "mov byte ptr [rsp+2], 0x52", // R
        "mov byte ptr [rsp+3], 0x0A", // \n
        "xor rax, rax", // SYS_LOG
        "lea rdi, [rsp]",
        "mov rsi, 4",
        "syscall",
        "2:",
        "mov rax, 1", // SYS_YIELD
        "syscall",
        "jmp 2b",
    );
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
#[link_section = ".user_text"]
pub unsafe extern "C" fn user_bootstrap_end() {
    core::arch::naked_asm!("nop");
}

static mut PENDING_PID: u32 = 0;
static mut PENDING_CR3: u64 = 0;
static mut PENDING_STACK: u64 = 0;
static mut PENDING_ENTRY: u64 = 0;

#[repr(C, align(16))]
struct TStack([u8; 8192]);
static mut TSTACK: TStack = TStack([0u8; 8192]);
static mut TSTACK_TOP: u64 = 0;

pub fn launch_first_userspace() -> Option<ProcessId> {
    Logger::log("≺USER≻ Creating bootstrap process...");

    // 1. Créer le processus
    let pid = process::create("init")?;

    // 2. Construire un ABO minimal à la volée depuis le code bootstrap naked
    let loaded = unsafe { load_bootstrap_code(pid)? };

    // 3. Stocker les paramètres pour le trampoline
    unsafe {
        PENDING_PID   = pid.0;
        PENDING_ENTRY = loaded.entry;
        PENDING_STACK = loaded.stack_top;
        PENDING_CR3   = process::get_cr3(pid)?;
        TSTACK_TOP    = TSTACK.0.as_ptr() as u64 + 8192;
    }

    // 4. Spawner le trampoline kernel
    if let Some(task_id)  = scheduler::spawn("init-trampoline", PriorityClass::Interactive, trampoline) {
        crate::scheduler::current::associate(task_id, pid);
    }

    Logger::log("≺USER≻ Bootstrap process queued");
    Some(pid)
}

/// Lancer un binaire ABO ou ELF depuis une tranche de mémoire kernel.
/// Utilisé par le runtime pour charger les services système.
pub fn launch_from_bytes(
    name: &'static str,
    data: &[u8],
    priority: PriorityClass,
) -> Option<ProcessId> {
    let pid = process::create(name)?;

    let loaded = loader::load(pid, data).ok()?;

    unsafe {
        PENDING_PID   = pid.0;
        PENDING_ENTRY = loaded.entry;
        PENDING_STACK = loaded.stack_top;
        PENDING_CR3   = process::get_cr3(pid)?;
        TSTACK_TOP    = TSTACK.0.as_ptr() as u64 + 8192;
    }

    if let Some(task_id)  = scheduler::spawn(name, priority, trampoline) {
        crate::scheduler::current::associate(task_id, pid);
    }
    Some(pid)
}

// ── Trampoline ring 0 → ring 3 ───────────────────────────────────────────────

fn trampoline() -> ! {
    let entry  = unsafe { PENDING_ENTRY };
    let stack  = unsafe { PENDING_STACK };
    let cr3    = unsafe { PENDING_CR3 };

    Logger::log("≺USER≻ Activating process...");

    unsafe {
        // Switcher sur la pile statique AVANT de changer cr3
        // (la pile du scheduler peut ne plus être accessible après)
        do_switch_and_iretq(cr3, entry, stack)
    }
}

#[unsafe(naked)]
unsafe extern "C" fn do_switch_and_iretq(cr3: u64, entry: u64, stack: u64) -> ! {
    core::arch::naked_asm!(
        // rdi = cr3, rsi = entry, rdx = stack

        // 1. Charger la pile statique
        "mov rsp, [{tstack_top}]",

        // 2. Changer cr3
        "mov cr3, rdi",

        // 3. Construire le frame iretq
        "mov rax, 0x1b",    // SS : user data ring 3
        "push rax",
        "push rdx",         // RSP user
        "mov rax, 0x202",   // RFLAGS : IF=1, bit1=1
        "push rax",
        "mov rax, 0x23",    // CS : user code ring 3
        "push rax",
        "push rsi",         // RIP : entry point

        // 4. Zéroïser les registres
        "xor rax, rax",
        "xor rbx, rbx",
        "xor rcx, rcx",
        "xor rdx, rdx",
        "xor rsi, rsi",
        "xor rdi, rdi",
        "xor rbp, rbp",
        "xor r8,  r8",
        "xor r9,  r9",
        "xor r10, r10",
        "xor r11, r11",
        "xor r12, r12",
        "xor r13, r13",
        "xor r14, r14",
        "xor r15, r15",

        "iretq",

        tstack_top = sym TSTACK_TOP,
    )
}

// ── Chargement du code bootstrap ─────────────────────────────────────────────

unsafe fn load_bootstrap_code(pid: ProcessId) -> Option<LoadedBinary> {
    use crate::mm::{pmm, vmm};
    use crate::loader::abo;

    let src_start = user_bootstrap     as *const u8;
    let src_end   = user_bootstrap_end as *const u8;
    let code_size = (src_end as usize)
        .saturating_sub(src_start as usize)
        .max(64)
        .min(4096);

    // Construire un ABO minimal en mémoire :
    //   header (64 bytes) + segment_entry (32 bytes) + code
    let seg_offset = (abo::ABO_HEADER_SIZE + 32) as u64;
    let total_size = seg_offset as usize + code_size;

    // Allouer un buffer temporaire sur la pile kernel (max 4 KiB + header)
    // On utilise une frame PMM comme buffer temporaire
    let buf_phys = pmm::alloc_frame()?;
    let buf_virt = vmm::phys_to_virt(buf_phys);
    let buf = core::slice::from_raw_parts_mut(buf_virt.as_mut_ptr::<u8>(), 4096);

    // Zéroïser
    core::ptr::write_bytes(buf.as_mut_ptr(), 0, 4096);

    // Header ABO
    let hdr = abo::build_header(
        [0x41,0x73,0x74,0x65,0x72,0x49,0x6e,0x69, // "AsterIni"
            0x74,0x00,0x00,0x00,0x00,0x00,0x00,0x00], // "t\0\0..."
        abo::ABO_FLAG_NATIVE,
        0, 0,                                        // pas de manifest
        abo::ABO_HEADER_SIZE as u32,                 // segments juste après le header
        1,                                           // 1 segment
        0,                                           // entry à offset 0 du segment
    );
    buf[..abo::ABO_HEADER_SIZE].copy_from_slice(&hdr);

    // Descripteur de segment : code exécutable en lecture seule
    let seg_entry = abo::build_segment_entry(
        crate::loader::USER_CODE_BASE,  // vaddr user
        code_size as u64,               // mem_size
        seg_offset,                     // file_off (dans le buffer ABO)
        code_size as u32,               // file_size
        abo::ABO_SEG_R | abo::ABO_SEG_X,
    );
    let seg_off = abo::ABO_HEADER_SIZE;
    buf[seg_off..seg_off + 32].copy_from_slice(&seg_entry);

    // Copier le code
    core::ptr::copy_nonoverlapping(
        src_start,
        buf[seg_offset as usize..].as_mut_ptr(),
        code_size,
    );

    // Charger via le loader ABO
    let result = abo::load(pid, &buf[..seg_offset as usize + code_size]);

    // Libérer le buffer temporaire
    pmm::free_frame(buf_phys);

    result.ok()
}