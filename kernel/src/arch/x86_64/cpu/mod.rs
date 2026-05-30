pub mod tss;
pub mod gdt;
pub mod stack;
pub mod percpu;

pub fn init() {
    unsafe {
        gdt::init();
    }
}