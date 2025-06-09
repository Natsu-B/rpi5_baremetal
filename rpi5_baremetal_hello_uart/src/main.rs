#![no_std]
#![no_main]

use core::{
    arch::{asm, global_asm},
    fmt::{self, Write},
    panic::PanicInfo,
};

unsafe extern "C" {
    static mut _BSS_START: usize;
    static mut _BSS_END: usize;
    static mut _STACK_TOP: usize;
}

const PL011_UART: usize = 0x10_7D00_1000;
const UART_DR: usize = 0x0;
const UART_FR: usize = 0x018;

// 最初に実行される部分 _startが最初に呼び出され、スタックの設定を行ったらresetに飛ぶ
global_asm!(
    r#"
.global _start
.section ".text.boot"

_start:
    ldr x0, =_STACK_TOP
    mov sp, x0
clear_bss:
    ldr x0, =_BSS_START
    ldr x1, =_BSS_END
clear_bss_loop:
    cmp x0, x1
    beq clear_bss_end
    str xzr, [x0], #8
    b clear_bss_loop
clear_bss_end:
    bl main
loop:
    wfe
    b loop
    "#
);

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

fn get_timer_counter() -> u64 {
    let counter;
    unsafe {
        asm!("
    isb
    mrs {counter}, CNTPCT_EL0
    ", counter = out(reg)counter)
    };
    counter
}

fn wait(current_frequency: u64, time: core::time::Duration) {
    let micros = time.as_micros();
    let start = get_timer_counter();
    let wait_time = u128::from(current_frequency / 1000 / 1000) * micros;
    while u128::from(get_timer_counter() - start) < wait_time {
        unsafe { asm!("nop") };
        core::hint::spin_loop();
    }
}

#[unsafe(no_mangle)]
extern "C" fn main() -> ! {
    let text = "HelloWorld!\r\nPL011\r\n";
    print_str(text, PL011_UART);
    let current_frequency;
    unsafe {
        asm!("mrs {current_frequency}, CNTFRQ_EL0", current_frequency = out(reg)current_frequency);
    }
    println!("system counter frequency: {}Hz", current_frequency);

    loop {
        println!("Hello");
        wait(current_frequency, core::time::Duration::from_secs(10));
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let _ = info;
    loop {
        unsafe { asm!("wfi") };
    }
}
