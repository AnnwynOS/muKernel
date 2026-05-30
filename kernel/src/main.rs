#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(naked_functions)]

use bootloader_api::{entry_point, BootInfo, BootloaderConfig};
use bootloader_api::config::Mapping;
use kernel::debug::log::Logger;
use kernel::arch::x86_64::halt::halt_loop;
use kernel::scheduler::{self, task::PriorityClass};

// Configuration bootloader embarquée dans le binaire kernel demandant explicitement le mapping phsique complet
static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    Logger::init();
    Logger::log("⟾ ✵✵✵⨑ µKernel ∱✵✵✵ ⟽");
    // Bordel, y'a tellement de caractères utf 8
    Logger::log("≺BOOT≻ Serial OK");

    unsafe { kernel::arch::x86_64::cpu::init(); }

    Logger::log("≺BOOT≻ Starting MM...");
    kernel::mm::init(boot_info);
    Logger::log("≺BOOT≻ MM OK");

    Logger::log("≺BOOT≻ Starting interrupts...");
    kernel::arch::x86_64::interrupts::init();
    Logger::log("≺BOOT≻ Interrupts OK");

    Logger::log("≺BOOT≻ Starting percpu...");
    unsafe {
        kernel::arch::x86_64::cpu::percpu::init(kernel::arch::x86_64::cpu::tss::kernel_stack_top());
    }
    Logger::log("≺BOOT≻ percpu OK...");

    Logger::log("≺BOOT≻ Starting syscalls...");
    unsafe { kernel::syscall::init(); }
    Logger::log("≺BOOT≻ Syscalls OK...");

    Logger::log("≺BOOT≻ Running self-tests...");
    self_test();

    Logger::log("≺BOOT≻ All systems nominal ; idle");

    Logger::log("≺BOOT≻ Spawning tasks...");

    scheduler::spawn("idle",    PriorityClass::Background,  task_idle);
    scheduler::spawn("worker",  PriorityClass::Productive,  task_worker);

    Logger::log("≺BOOT≻ Starting scheduler...");
    scheduler::start();

    Logger::log("≺BOOT≻ Entering idle");
    task_idle()
}

fn task_idle() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

fn task_worker() -> ! {
    Logger::log("≺WORKER≻ task_worker started");
    loop {
        x86_64::instructions::hlt();
    }
}

fn self_test() {
    use kernel::capabilities::{self, CapabilityKind, Rights};
    use kernel::ipc;

    let cap = capabilities::create(CapabilityKind::Service { service_id: 1 }, Rights::READ, 2);
    assert!(cap.is_some());
    let id = cap.unwrap();
    assert!(capabilities::check(id, Rights::READ));
    assert!(!capabilities::check(id, Rights::WRITE));
    capabilities::revoke(id);
    assert!(!capabilities::check(id, Rights::READ));
    assert!(ipc::create_endpoint().is_some());

    let pid = kernel::process::create("test-proc");
    assert!(pid.is_some());
    Logger::log("≺SELFTEST≻ Process isolation OK");

    Logger::log("≺SELFTEST≻ OK");
}