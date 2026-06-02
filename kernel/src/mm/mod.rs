pub mod pmm;
pub mod vmm;
pub mod user_ptr;

use bootloader_api::info::BootInfo;
use crate::debug::log::Logger;

pub fn init(boot_info: &BootInfo) {
    Logger::log("≺MM≻ Initializing memory subsystem...");

    // PMM : inventaire des frames physiques disponibles
    pmm::init(&boot_info.memory_regions);

    // VMM : activer le mapping virtuel
    vmm::init(boot_info);

    Logger::log("≺MM≻ Memory subsystem ready");
}
