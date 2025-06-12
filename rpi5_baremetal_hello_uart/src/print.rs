use crate::interfaces::pl011::Pl011Uart;
use core::{
    cell::OnceCell,
    fmt::{self, Write},
};

pub const DEBUG_UART: OnceCell<Pl011Uart> = OnceCell::new();

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
        if let Some(writer) = DEBUG_UART.get() {
            writer.write(s);
        }
        Ok(())
    }
}
