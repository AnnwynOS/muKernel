use core::cell::UnsafeCell;
use x86_64::registers::model_specific::{GsBase, KernelGsBase};
use x86_64::VirtAddr;

#[repr(C)]
pub struct PercpuData {
    pub user_rsp: u64,
    pub kernel_rsp: u64,
}

impl PercpuData {
    const fn new() -> Self {
        Self { user_rsp: 0, kernel_rsp: 0 }
    }
}

struct PercpuWrap(UnsafeCell<PercpuData>);
unsafe impl Sync for PercpuWrap {}

static PERCPU: PercpuWrap = PercpuWrap(UnsafeCell::new(PercpuData::new()));

pub unsafe fn init(kernel_stack_top: u64) {
    let data = &mut *PERCPU.0.get();
    data.kernel_rsp = kernel_stack_top;

    GsBase::write(VirtAddr::new(data as *mut PercpuData as u64));
    
    KernelGsBase::write(VirtAddr::new(data as *mut PercpuData as u64));
}