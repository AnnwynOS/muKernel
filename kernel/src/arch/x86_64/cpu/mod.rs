pub mod tss;
pub mod gdt;
pub mod stack;

pub fn init() {
    unsafe {
        gdt::init();
    }
}