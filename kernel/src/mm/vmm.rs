use bootloader_api::info::BootInfo;
use x86_64::{
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTable,
        PageTableFlags, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

use crate::debug::log::Logger;
use super::pmm;

struct PmmFrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for PmmFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        pmm::alloc_frame()
            .map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

static mut PHYSICAL_MEMORY_OFFSET: u64 = 0;
static mut VMM_READY: bool = false;

pub fn phys_to_virt(phys: u64) -> VirtAddr {
    VirtAddr::new(unsafe { PHYSICAL_MEMORY_OFFSET } + phys)
}

pub fn physical_memory_offset() -> u64 {
    unsafe{ PHYSICAL_MEMORY_OFFSET }
}

pub fn init(boot_info: &BootInfo) {
    Logger::log("≺VMM≻ Checking physical_memory_offset...");

    let offset = match boot_info.physical_memory_offset.into_option() {
        Some(o) => o,
        None => {
            // Le bootloader n'a pas mappé toute la mémoire physique, on continue sans VMM, le PMM suffira. Mapping de pages recquiert alors l'offseft.
            Logger::log("≺VMM≻ WARNING: offset not provided");
            return;
        }
    };

    Logger::log("≺VMM≻ Offset found");
    unsafe {
        PHYSICAL_MEMORY_OFFSET = offset;
        VMM_READY = true;
    }
    Logger::log("≺VMM≻ Ready");
}

pub fn is_ready() -> bool {
    unsafe { VMM_READY }
}

pub unsafe fn map_page(
    virt: VirtAddr,
    phys: PhysAddr,
    flags: PageTableFlags,
) -> Result<(), &'static str> {
    if !VMM_READY { return Err("VMM not initialized"); }

    let page: Page<Size4KiB> = Page::containing_address(virt);
    let frame = PhysFrame::containing_address(phys);
    let phys_offset = VirtAddr::new(PHYSICAL_MEMORY_OFFSET);
    let l4_table = active_l4_table(phys_offset);
    let mut mapper = OffsetPageTable::new(l4_table, phys_offset);
    let mut fa = PmmFrameAllocator;

    mapper
        .map_to(page, frame, flags, &mut fa)
        .map_err(|_| "map_to failed")?
        .flush();

    Ok(())
}

unsafe fn active_l4_table(phys_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;
    let (l4_frame, _) = Cr3::read();
    let phys = l4_frame.start_address();
    let virt = phys_offset + phys.as_u64();
    &mut *virt.as_mut_ptr::<PageTable>()
}