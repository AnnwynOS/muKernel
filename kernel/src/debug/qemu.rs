pub fn exit_qemu(success: bool) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(if success { 0x10u32 } else { 0x11u32 });
    }
}