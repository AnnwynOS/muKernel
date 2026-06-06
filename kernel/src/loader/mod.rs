//! Loader unifié entre ABO natif, ELF pour la compatibilité, et même WASM pour plus tard

pub mod abo;
pub mod elf;
mod segments;

use x86_64::structures::paging::PageTableFlags;
use x86_64::VirtAddr;
use crate::process::ProcessId;
use crate::mm::user_ptr::USER_SPACE_END;

#[derive(Debug, Clone, Copy)]
pub struct LoadedBinary {
    pub entry: u64,
    pub stack_top: u64,
    pub brk: u64,
}

#[derive(Debug, Clone, Copy)]
pub enum LoadError {
    TooSmall,
    BadMagic,
    UnsupportedFormat,
    UnsupportedArch,
    UnsupportedVersion,
    BadSegmentTable,
    SegmentOutOfFile,
    VaddrInKernelSpace,
    SegmentOverflow,
    OutOfFrames,
    MappingFailed,
    StackMappingFailed,
    MissingCapability,
    InvalidEntryPoint,
    ProcessNotFound,
}

pub fn load(pid: ProcessId, data: &[u8]) -> Result<LoadedBinary, LoadError> {
    if data.len() < 4 { return Err(LoadError::TooSmall); }

    match &data[..4] {
        b"ABO\0"  => abo::load(pid, data),
        [0x7f, b'E', b'L', b'F'] => elf::load(pid, data),
        _ => Err(LoadError::UnsupportedFormat),
    }
}
pub const USER_CODE_BASE: u64 = 0x0040_0000;
pub const USER_STACK_BASE: u64 = 0x0000_7FFF_F000_0000;
pub const USER_STACK_SIZE: u64 = 0x2_0000;
pub const USER_STACK_TOP: u64 = USER_STACK_BASE + USER_STACK_SIZE;

pub fn jump_to_userspace(loaded: LoadedBinary) -> ! {
    let entry = loaded.entry;
    let stack_top = loaded.stack_top;

    unsafe {
        core::arch::asm!(
        "and rsp, -16",

        "mov rax, 0x1b",
        "push rax",
        "push {stack}",
        "mov rax, 0x202",
        "push rax",
        "mov rax, 0x23",
        "push rax",
        "push {entry}",

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

        entry = in(reg) entry,
        stack = in(reg) stack_top,
        options(noreturn),
        )
    }
}