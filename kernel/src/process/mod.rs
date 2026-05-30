use x86_64::{
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTable,
        PageTableFlags, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

use crate::mm::pmm;
use crate::mm::vmm;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Running,
    Suspended,
    Zombie,
}

pub const USER_STACK_TOP:  u64 = 0x0000_7FFF_FFFF_F000;
pub const USER_STACK_SIZE: u64 = 0x10000;
pub const USER_CODE_BASE:  u64 = 0x0000_0000_0040_0000; // 4 MB

pub struct Process {
    pub id: ProcessId,
    pub state: ProcessState,
    pub name: &'static str,

    pub cr3:   PhysAddr,

    l4_table:  *mut PageTable,
}

unsafe impl Send for Process {}

impl Process {
    pub fn new(id: ProcessId, name: &'static str) -> Option<Self> {
        let l4_phys = pmm::alloc_frame()?;
        let l4_phys = PhysAddr::new(l4_phys);

        let l4_virt = vmm::phys_to_virt(l4_phys.as_u64());
        let l4_table = l4_virt.as_mut_ptr::<PageTable>();

        unsafe {
            (*l4_table).zero();

            let current_l4 = current_l4_table();
            for i in 256..512 {
                (&mut *l4_table)[i] = (&*current_l4)[i].clone();
            }
        }

        Some(Process {
            id,
            state: ProcessState::Running,
            name,
            cr3: l4_phys,
            l4_table,
        })
    }

    pub fn map_user_page(
        &mut self,
        virt: VirtAddr,
        phys: PhysAddr,
        flags: PageTableFlags,
    ) -> Result<(), &'static str> {
        let flags = flags | PageTableFlags::USER_ACCESSIBLE;

        unsafe {
            let phys_offset = VirtAddr::new(vmm::physical_memory_offset());
            let mut mapper = OffsetPageTable::new(&mut *self.l4_table, phys_offset);
            let mut fa = PmmFrameAllocator;

            let page: Page<Size4KiB> = Page::containing_address(virt);
            let frame = PhysFrame::containing_address(phys);

            mapper
                .map_to(page, frame, flags, &mut fa)
                .map_err(|_| "map_user_page failed")?
                .flush();
        }
        Ok(())
    }

    pub fn alloc_user_pages(
        &mut self,
        virt_base: VirtAddr,
        num_pages: usize,
        flags: PageTableFlags,
    ) -> Result<(), &'static str> {
        for i in 0..num_pages {
            let phys = pmm::alloc_frame()
                .ok_or("PMM out of frames")?;
            let virt = virt_base + (i as u64 * 4096);
            self.map_user_page(virt, PhysAddr::new(phys), flags)?;
        }
        Ok(())
    }

    pub unsafe fn activate(&self) {
        use x86_64::registers::control::{Cr3, Cr3Flags};
        let frame = PhysFrame::containing_address(self.cr3);
        Cr3::write(frame, Cr3Flags::empty());
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        // TODO : libérer récursivement toutes les frames allouées
    }
}

unsafe fn current_l4_table() -> *mut PageTable {
    use x86_64::registers::control::Cr3;
    let (frame, _) = Cr3::read();
    let phys = frame.start_address();
    let virt = vmm::phys_to_virt(phys.as_u64());
    virt.as_mut_ptr::<PageTable>()
}

struct PmmFrameAllocator;
unsafe impl FrameAllocator<Size4KiB> for PmmFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        pmm::alloc_frame()
            .map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

use spin::Mutex;

const MAX_PROCESSES: usize = 16;

struct ProcessTable {
    procs:[Option<Process>; MAX_PROCESSES],
    next_id: u32,
}

impl ProcessTable {
    const fn new() -> Self {
        Self {
            procs: [
                None, None, None, None,
                None, None, None, None,
                None, None, None, None,
                None, None, None, None,
            ],
            next_id: 1,
        }
    }

    fn insert(&mut self, proc: Process) -> ProcessId {
        let id = proc.id;
        for slot in self.procs.iter_mut() {
            if slot.is_none() {
                *slot = Some(proc);
                return id;
            }
        }
        panic!("process table full");
    }

    fn get_mut(&mut self, id: ProcessId) -> Option<&mut Process> {
        self.procs.iter_mut().flatten()
            .find(|p| p.id == id)
    }

    fn next_id(&mut self) -> ProcessId {
        let id = ProcessId(self.next_id);
        self.next_id += 1;
        id
    }
}

static PROCESSES: Mutex<ProcessTable> = Mutex::new(ProcessTable::new());

pub fn create(name: &'static str) -> Option<ProcessId> {
    let mut table = PROCESSES.lock();
    let id = table.next_id();
    let proc = Process::new(id, name)?;
    Some(table.insert(proc))
}

pub fn map_pages(
    pid: ProcessId,
    virt_base: VirtAddr,
    num_pages: usize,
    flags: PageTableFlags,
) -> Result<(), &'static str> {
    let mut table = PROCESSES.lock();
    let proc = table.get_mut(pid).ok_or("process not found")?;
    proc.alloc_user_pages(virt_base, num_pages, flags)
}

pub unsafe fn activate(pid: ProcessId) {
    let mut table = PROCESSES.lock();
    if let Some(proc) = table.get_mut(pid) {
        proc.activate();
    }
}