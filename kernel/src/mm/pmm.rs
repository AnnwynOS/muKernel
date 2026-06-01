use bootloader_api::info::{MemoryRegionKind, MemoryRegions};
use spin::Mutex;
use crate::debug::log::Logger;

const PAGE_SIZE:  u64   = 4096;
const MAX_FRAMES: usize = 512 * 1024; // 2 GB
const BITMAP_LEN: usize = MAX_FRAMES / 64;

struct Pmm {
    bits: [u64; BITMAP_LEN],
    free_count: usize,
}

impl Pmm {
    const fn new() -> Self {
        Self { bits: [0u64; BITMAP_LEN], free_count: 0 }
    }

    fn set_free(&mut self, frame: usize) {
        if frame >= MAX_FRAMES { return; }
        let (w, b) = (frame / 64, frame % 64);
        if (self.bits[w] >> b) & 1 == 0 {
            self.bits[w] |= 1 << b;
            self.free_count += 1;
        }
    }

    fn set_used(&mut self, frame: usize) {
        if frame >= MAX_FRAMES { return; }
        let (w, b) = (frame / 64, frame % 64);
        if (self.bits[w] >> b) & 1 == 1 {
            self.bits[w] &= !(1u64 << b);
            self.free_count -= 1;
        }
    }

    fn alloc(&mut self) -> Option<usize> {
        // Sauter les 512 premiers frames ; les 2 MiB réservée en zone BIOS)
        for w in 8..BITMAP_LEN {
            if self.bits[w] == 0 { continue; }
            let b = self.bits[w].trailing_zeros() as usize;
            let frame = w * 64 + b;
            self.set_used(frame);
            return Some(frame);
        }
        None
    }

    fn free(&mut self, frame: usize) {
        self.set_free(frame);
    }
}

static PMM: Mutex<Pmm> = Mutex::new(Pmm::new());

pub fn init(regions: &MemoryRegions) {
    Logger::log("≺PMM≻ Initializing...");
    let mut pmm = PMM.lock();
    for region in regions.iter() {
        if region.kind != MemoryRegionKind::Usable { continue; }
        let start= ((region.start / PAGE_SIZE) as usize).max(512);
        let end= (region.end / PAGE_SIZE).min(MAX_FRAMES as u64) as usize;
        for frame in start..end {
            pmm.set_free(frame);
        }
    }
    Logger::log("≺PMM≻ Ready");
}

pub fn alloc_frame() -> Option<u64> {
    PMM.lock().alloc().map(|f| f as u64 * PAGE_SIZE)
}

pub fn free_frame(phys: u64) {
    PMM.lock().free((phys / PAGE_SIZE) as usize);
}

pub fn free_frames() -> usize {
    PMM.lock().free_count
}