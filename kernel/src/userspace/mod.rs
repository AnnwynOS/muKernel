use x86_64::{
    structures::paging::{PageTableFlags, PageTable},
    VirtAddr,
};

use crate::process::{self, ProcessId, USER_CODE_BASE, USER_STACK_TOP, USER_STACK_SIZE};
use crate::mm::vmm;
use crate::scheduler::{self, task::PriorityClass};
use crate::debug::log::Logger;

#[unsafe(naked)]
#[unsafe(no_mangle)]
#[link_section = ".user_text"]
pub unsafe extern "C" fn user_entry() {
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
pub unsafe extern "C" fn user_entry_end() {
    core::arch::naked_asm!("nop");
}

pub fn launch_first_userspace() -> Option<ProcessId> {
    Logger::log("≺USER≻ Creating process...");
    let pid = process::create("init")?;

    let code_flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
    process::map_pages(pid, VirtAddr::new(USER_CODE_BASE), 1, code_flags).ok()?;
    unsafe { copy_code_to_user(pid)?; }

    let stack_flags = PageTableFlags::PRESENT
        | PageTableFlags::WRITABLE
        | PageTableFlags::USER_ACCESSIBLE
        | PageTableFlags::NO_EXECUTE;
    let stack_base = USER_STACK_TOP - USER_STACK_SIZE;
    process::map_pages(pid, VirtAddr::new(stack_base),
                       (USER_STACK_SIZE / 4096) as usize, stack_flags).ok()?;

    unsafe { PENDING_USER_PID = pid.0; }
    scheduler::spawn("init-trampoline", PriorityClass::Interactive, ring3_trampoline);

    Logger::log("≺USER≻ Process queued");
    Some(pid)
}

static mut PENDING_USER_PID: u32 = 0;
static mut USER_CR3: u64 = 0;
static mut TSTACK_TOP: u64 = 0;

#[repr(C, align(16))]
struct TStack([u8; 8192]);
static mut TSTACK: TStack = TStack([0u8; 8192]);

fn ring3_trampoline() -> ! {
    let pid = ProcessId(unsafe { PENDING_USER_PID });

    let cr3 = process::get_cr3(pid).unwrap();

    unsafe {
        USER_CR3   = cr3;
        TSTACK_TOP = TSTACK.0.as_ptr() as u64 + 8192;
    }

    Logger::log("≺USER≻ Jumping to ring 3...");

    unsafe { trampoline_naked() }
}

#[unsafe(naked)]
unsafe extern "C" fn trampoline_naked() -> ! {
    core::arch::naked_asm!(
        "mov rsp, [{tstack_top}]",

        "mov rax, [{user_cr3}]",
        "mov cr3, rax",

        "mov rax, 0x1b",
        "push rax",
        "mov rax, {ustack}",
        "push rax",
        "mov rax, 0x202",
        "push rax",
        "mov rax, 0x23",
        "push rax",
        "mov rax, {entry}",
        "push rax",

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
        user_cr3   = sym USER_CR3,
        entry      = const USER_CODE_BASE,
        ustack     = const USER_STACK_TOP - 16,
    );
}

unsafe fn copy_code_to_user(pid: ProcessId) -> Option<()> {
    let src_start = user_entry as *const u8;
    let src_end = user_entry_end as *const u8;
    let size = (src_end as usize).saturating_sub(src_start as usize).max(64).min(4096);
    let phys = walk_page(pid, USER_CODE_BASE)?;
    let dst= vmm::phys_to_virt(phys).as_mut_ptr::<u8>();
    core::ptr::copy_nonoverlapping(src_start, dst, size);
    Logger::log("≺USER≻ Code copied");
    Some(())
}

unsafe fn walk_page(pid: ProcessId, virt: u64) -> Option<u64> {
    let cr3_phys = process::get_cr3(pid)?;
    let virt= VirtAddr::new(virt);
    macro_rules! next_table {
        ($entry:expr) => {{
            let e = $entry;
            if !e.flags().contains(PageTableFlags::PRESENT) { return None; }
            &*(vmm::phys_to_virt(e.addr().as_u64()).as_ptr::<PageTable>())
        }};
    }
    let l4 = &*(vmm::phys_to_virt(cr3_phys).as_ptr::<PageTable>());
    let l3 = next_table!(&l4[virt.p4_index()]);
    let l2 = next_table!(&l3[virt.p3_index()]);
    let l1 = next_table!(&l2[virt.p2_index()]);
    let e = &l1[virt.p1_index()];
    if !e.flags().contains(PageTableFlags::PRESENT) { return None; }
    Some(e.addr().as_u64())
}