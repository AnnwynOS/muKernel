#![no_std]
#![no_main]

use bootloader_api::{entry_point, BootInfo};

use kernel::debug::log::Logger;
use kernel::arch::x86_64::halt::halt_loop;

entry_point!(kernel_main);

fn kernel_main(_boot_info: &'static mut BootInfo) -> ! {
    Logger::init();

    Logger::log("Kernel booting...");
    Logger::log("Debug system online");

    halt_loop();
}