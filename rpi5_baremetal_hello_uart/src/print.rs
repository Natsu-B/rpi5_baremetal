use core::fmt::{self, Write};

const PL011_UART: usize = 0x10_7D00_1000;
const UART_DR: usize = 0x0;
const UART_FR: usize = 0x018;

fn write_char(c: char, addr: usize) {
    unsafe { core::ptr::write_volatile((addr + UART_DR) as *mut u32, c as u32) };
}

fn is_write_fifo_full(addr: usize) -> bool {
    (unsafe { core::ptr::read_volatile((addr + UART_FR) as *const u16) } & (1 << 5)) != 0
}

fn print_str(text: &str, address: usize) {
    for c in text.chars() {
        while is_write_fifo_full(address) {}
        write_char(c, address);
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

pub fn _print(args: fmt::Arguments) {
    let mut writer = UartWriter {};
    writer.write_fmt(args).unwrap();
}

struct UartWriter;

impl Write for UartWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        print_str(s, PL011_UART);
        Ok(())
    }
}