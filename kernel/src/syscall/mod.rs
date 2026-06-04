use x86_64::registers::model_specific::{Star, LStar, SFMask};
use x86_64::registers::rflags::RFlags;
use x86_64::VirtAddr;
use crate::debug::log::Logger;
use crate::ipc;
use crate::mm::user_ptr;

pub const SYS_LOG: u64 = 0;
pub const SYS_YIELD: u64 = 1;
pub const SYS_EXIT: u64 = 2;
pub const SYS_IPC_SEND: u64 = 3;
pub const SYS_IPC_RECV: u64 = 4;
pub const SYS_CAP_CREATE: u64 = 5;
pub const SYS_CAP_REVOKE: u64 = 6;
pub const SYS_MEM_MAP: u64 = 7;
pub const SYS_ENDPOINT_CREATE: u64 = 8;

pub unsafe fn init() {
    Logger::log("≺SYSCALL≻ Initializing...");
    Star::write(
        x86_64::structures::gdt::SegmentSelector::new(4, x86_64::PrivilegeLevel::Ring3),
        x86_64::structures::gdt::SegmentSelector::new(3, x86_64::PrivilegeLevel::Ring3),
        x86_64::structures::gdt::SegmentSelector::new(1, x86_64::PrivilegeLevel::Ring0),
        x86_64::structures::gdt::SegmentSelector::new(2, x86_64::PrivilegeLevel::Ring0),
    ).expect("STAR write failed");

    LStar::write(VirtAddr::new(syscall_entry as *const () as u64));

    SFMask::write(RFlags::INTERRUPT_FLAG);

    // Activer SCE (System Call Extensions) dans EFER
    use x86_64::registers::model_specific::Efer;
    use x86_64::registers::model_specific::EferFlags;
    Efer::update(|f| *f |= EferFlags::SYSTEM_CALL_EXTENSIONS);

    Logger::log("≺SYSCALL≻ OK");
}

#[unsafe(naked)]
unsafe extern "C" fn syscall_entry() {
    core::arch::naked_asm!(
        "swapgs",
        "mov gs:[0], rsp",
        "mov rsp, gs:[8]",

        "push rcx",
        "push r11",

        "sti",

        "mov r9, r8",
        "mov r8, r10",
        "mov rcx, rdx",
        "mov rdx, rsi",
        "mov rsi, rdi",
        "mov rdi, rax",

        "call {dispatcher}",

        "cli",

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
    nr: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
) -> i64 {
    match nr {
        SYS_LOG => sys_log(arg1, arg2),
        SYS_YIELD => sys_yield(),
        SYS_EXIT => sys_exit(arg1),
        SYS_IPC_SEND => sys_ipc_send(arg1, arg2, arg3),
        SYS_IPC_RECV => sys_ipc_recv(arg1, arg2, arg3, arg4),
        SYS_CAP_REVOKE => sys_cap_revoke(arg1),
        SYS_ENDPOINT_CREATE => sys_endpoint_create(arg1),
        _ => -1, // ENOSYS
    }
}

unsafe fn sys_log(ptr: u64, len: u64) -> i64 {
    let len = len.min(256) as usize;

    let slice = match user_ptr::validate_user_str(ptr, len.min(256)) {
        Ok(s) => s,
        Err(_) => return -14,
    };

    for &b in slice {
        crate::debug::serial::Serial::write_byte(b);
    }
    slice.len() as i64
}

fn sys_yield() -> i64 {
    // il faudra forcer un reschedule immédiat
    0
}

fn sys_exit(code: u64) -> ! {
    Logger::log("≺SYSCALL≻ SYS_EXIT");

    if let Some(pid) = crate::scheduler::current::current_process() {
        crate::process::mark_exit(pid);
    }

    crate::arch::x86_64::halt::halt_loop();
}

unsafe fn sys_ipc_send(cap_id: u64, endpoint_id: u64, data_ptr: u64) -> i64 {
    use crate::capabilities::CapabilityId;
    use crate::ipc::{self, EndpointId, Message, MessageKind};

    let cap = CapabilityId(cap_id);
    let ep  = EndpointId(endpoint_id);

    let data_slice: &[u8] = if data_ptr != 0 {
        match user_ptr::validate_user_str(data_ptr, 48) {
            Ok(s) => s,
            Err(_) => return -14,
        }
    } else {
        &[]
    };

    let sender = crate::scheduler::current::current_process()
        .map(|p| p.0 as u64)
        .unwrap_or(0);

    let msg = Message::with_data(MessageKind::Request, sender, data_slice);

    match ipc::send(cap, ep, msg) {
        Ok(()) => 0,
        Err(ipc::IpcError::PermissionDenied) => -1,
        Err(_) => -11,
    }
}

fn sys_ipc_recv(buf_ptr: u64, endpoint_id: u64, buf_len: u64, out_len_ptr: u64) -> i64 {
    use crate::capabilities::CapabilityId;
    use crate::ipc::{self, EndpointId};

    let len = buf_len as usize;
    if len > 0 {
        if user_ptr::validate_user_write(buf_ptr, len.min(48)).is_err() {
            return -14;
        }
    }

    let ep  = EndpointId(endpoint_id);

    match ipc::recv_unchecked(ep) {
        Ok(msg) => {
            let copy_len = (msg.data_len as usize).min(len);
            if copy_len > 0 {
                unsafe { core::ptr::copy_nonoverlapping(msg.data.as_ptr(), buf_ptr as *mut u8, copy_len); }
            }
            if out_len_ptr != 0 {
                if user_ptr::validate_user_write(out_len_ptr, 4).is_ok() {
                    unsafe { core::ptr::write_unaligned(out_len_ptr as *mut u32, copy_len as u32); }
                }
            }
            0
        },
        Err(_)   => -11,
    }
}

fn sys_cap_revoke(cap_id: u64) -> i64 {
    use crate::capabilities::{self, CapabilityId};
    if capabilities::revoke(CapabilityId(cap_id)) { 0 } else { -1 }
}

fn sys_endpoint_create(_flags: u64)-> i64 {
    use crate::ipc;
    match ipc::create_endpoint() {
        Some((ep, _send, _recv)) => ep.0 as i64,
        None => -12,
    }
}

unsafe fn sys_mem_map(_base: u64, _len: u64, _flags: u64) -> i64 { -1 }