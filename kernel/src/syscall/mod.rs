use x86_64::registers::model_specific::{Msr, Star, LStar, SFMask};
use x86_64::registers::rflags::RFlags;
use x86_64::VirtAddr;
use crate::debug::log::Logger;

pub const SYS_LOG: u64 = 0;
pub const SYS_YIELD: u64 = 1;
pub const SYS_EXIT: u64 = 2;
pub const SYS_IPC_SEND: u64 = 3;
pub const SYS_IPC_RECV: u64 = 4;
pub const SYS_CAP_CREATE: u64 = 5;
pub const SYS_CAP_REVOKE: u64 = 6;
pub const SYS_MEM_MAP: u64 = 7;

pub unsafe fn init() {
    Logger::log("≺SYSCALL≻ Initializing...");
    Star::write(
        x86_64::structures::gdt::SegmentSelector::new(4, x86_64::PrivilegeLevel::Ring3),
        x86_64::structures::gdt::SegmentSelector::new(3, x86_64::PrivilegeLevel::Ring3),
        x86_64::structures::gdt::SegmentSelector::new(1, x86_64::PrivilegeLevel::Ring0),
        x86_64::structures::gdt::SegmentSelector::new(2, x86_64::PrivilegeLevel::Ring0),
    ).expect("STAR write failed");

    LStar::write(VirtAddr::new(syscall_handler_naked as *const () as u64));

    SFMask::write(RFlags::INTERRUPT_FLAG);

    // Activer SCE (System Call Extensions) dans EFER
    use x86_64::registers::model_specific::Efer;
    use x86_64::registers::model_specific::EferFlags;
    Efer::update(|f| *f |= EferFlags::SYSTEM_CALL_EXTENSIONS);

    Logger::log("≺SYSCALL≻ OK");
}

#[unsafe(naked)]
unsafe extern "C" fn syscall_handler_naked() {
    core::arch::naked_asm!(
        "swapgs",
        "mov gs:[0], rsp",
        "mov rsp, gs:[8]",

        "push rcx",
        "push r11",
        "push rbp",
        "push r12",
        "push r13",
        "push r14",
        "push r15",

        "sti",

        "mov rcx, r10",
        "call {dispatcher}",

        "cli",

        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbp",
        "pop r11", 
        "pop rcx", 

        // Restaurer RSP user
        "mov rsp, gs:[0]",
        "swapgs",

        // Retour en ring 3
        "sysretq",

        dispatcher = sym syscall_dispatcher,
    );
}

unsafe extern "C" fn syscall_dispatcher(
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
    _arg6: u64,
) -> i64 {
    let syscall_nr: u64;
    core::arch::asm!("", out("rax") syscall_nr);

    match syscall_nr {
        SYS_LOG => sys_log(arg1, arg2),
        SYS_YIELD => sys_yield(),
        SYS_EXIT => sys_exit(arg1),
        SYS_IPC_SEND => sys_ipc_send(arg1, arg2, arg3),
        SYS_IPC_RECV => sys_ipc_recv(arg1, arg2),
        SYS_CAP_REVOKE => sys_cap_revoke(arg1),
        _ => -1, // ENOSYS
    }
}

unsafe fn sys_log(ptr: u64, len: u64) -> i64 {
    let len = len.min(256) as usize;
    // TODO : valider que [ptr, ptr+len) est dans l'espace user
    let slice = core::slice::from_raw_parts(ptr as *const u8, len);
    for &b in slice {
        crate::debug::serial::Serial::write_byte(b);
    }
    len as i64
}

fn sys_yield() -> i64 {
    // il faudra forcer un reschedule immédiat
    0
}

fn sys_exit(code: u64) -> i64 {
    Logger::log("≺SYSCALL≻ SYS_EXIT");
    // TODO : marquer le processus comme Zombiepuis  nettoyer les ressources
    crate::arch::x86_64::halt::halt_loop();
}

unsafe fn sys_ipc_send(cap_id: u64, endpoint_id: u64, data_ptr: u64) -> i64 {
    use crate::capabilities::CapabilityId;
    use crate::ipc::{self, EndpointId, Message, MessageKind};

    let cap = CapabilityId(cap_id);
    let ep  = EndpointId(endpoint_id);
    let msg = Message::empty(MessageKind::Request, 0); // TODO : copier data_ptr

    match ipc::send(cap, ep, msg) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

fn sys_ipc_recv(cap_id: u64, endpoint_id: u64) -> i64 {
    use crate::capabilities::CapabilityId;
    use crate::ipc::{self, EndpointId};

    let cap = CapabilityId(cap_id);
    let ep  = EndpointId(endpoint_id);

    match ipc::recv(cap, ep) {
        Ok(_msg) => 0,
        Err(_)   => -1,
    }
}

fn sys_cap_revoke(cap_id: u64) -> i64 {
    use crate::capabilities::{self, CapabilityId};
    if capabilities::revoke(CapabilityId(cap_id)) { 0 } else { -1 }
}

unsafe fn sys_mem_map(_base: u64, _len: u64, _flags: u64) -> i64 { -1 }