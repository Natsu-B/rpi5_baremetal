use core::fmt::{self, Write};

use crate::PL011_UART_ADDR;
use crate::interfaces::pl011::Pl011Uart;
use crate::spinlock::SpinLock;

static DEBUG_UART: SpinLock<Option<Pl011Uart>> = SpinLock::new(None);
static RP1_UART0: SpinLock<Option<Pl011Uart>> = SpinLock::new(None);

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::print::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => ({
        // 改行を含めてprint!を呼び出す
        $crate::print!("{}\n", format_args!($($arg)*));
    });
}

pub fn _print(args: fmt::Arguments) {
    let mut debug_uart_cell = DEBUG_UART.lock();
    let uart = debug_uart_cell.get_or_insert_with(|| Pl011Uart::new(PL011_UART_ADDR));
    uart.write_fmt(args).unwrap();

    let mut rp1_uart0_cell = RP1_UART0.lock();
    if let Some(uart) = rp1_uart0_cell.as_mut() {
        uart.write_fmt(args).unwrap();
    }
}
