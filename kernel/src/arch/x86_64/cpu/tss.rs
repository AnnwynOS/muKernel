use x86_64::structures::tss::TaskStateSegment;
use super::stack::Stack;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;
pub const NMI_IST_INDEX: u16 = 1;
#[repr(C, align(16))]
struct CpuStacks {
    kernel: Stack,
    double_fault: Stack,
    nmi: Stack,
}

static mut CPU_STACKS: CpuStacks = CpuStacks {
    kernel: Stack::new(),
    double_fault: Stack::new(),
    nmi: Stack::new(),
};

pub static mut TSS: TaskStateSegment = TaskStateSegment::new();

pub unsafe fn init() {
    TSS.privilege_stack_table[0] =
        CPU_STACKS.kernel.top();
    TSS.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] =
        CPU_STACKS.double_fault.top();
    TSS.interrupt_stack_table[NMI_IST_INDEX as usize] =
        CPU_STACKS.nmi.top();
}

pub fn kernel_stack_top() -> u64 {
    unsafe { CPU_STACKS.kernel.top().as_u64() }
}