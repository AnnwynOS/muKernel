use crate::debug::log::Logger;
use core::panic::PanicInfo;

#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    Logger::log("KERNEL PANIC");
    Logger::log("panic message received");


    loop {
        x86_hlt();
    }
}

fn x86_hlt() {
    unsafe {
        core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
    }
}