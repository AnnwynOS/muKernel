use crate::debug::serial::Serial;

pub struct Logger;

impl Logger {
    pub fn init() {
        Serial::init();
    }

    pub fn log(msg: &str) {
        for b in msg.bytes() {
            Serial::write_byte(b);
        }
        Serial::write_byte(b'\n');
    }
}