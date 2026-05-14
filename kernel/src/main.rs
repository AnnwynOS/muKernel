#![no_std]
#![no_main]

use bootloader_api::{entry_point, BootInfo, BootloaderConfig};
use bootloader_api::config::Mapping;

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    if let Some(fb) = boot_info.framebuffer.as_mut() {
        let info = fb.info();
        let buf  = fb.buffer_mut();

        for chunk in buf.chunks_mut(info.bytes_per_pixel) {
            if chunk.len() >= 3 {
                chunk[0] = 0xCC; // B
                chunk[1] = 0x44; // G
                chunk[2] = 0x00; // R
            }
            if chunk.len() >= 4 {
                chunk[3] = 0xFF; // A
            }
        }
    }

    loop {
        x86_hlt();
    }
}
#[inline(always)]
fn x86_hlt() {
    unsafe { core::arch::asm!("hlt", options(nomem, nostack, preserves_flags)) }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        x86_hlt();
    }
}