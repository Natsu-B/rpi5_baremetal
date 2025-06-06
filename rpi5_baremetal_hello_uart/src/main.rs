#![no_std]
#![no_main]

use core::{
    arch::{asm, global_asm},
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

fn print_u8(text: &'static str, address: usize) {
    for c in text.chars() {
        while is_write_fifo_full(address) {}
        write_char(c, address);
    }
}

#[unsafe(no_mangle)]
extern "C" fn main() -> ! {
    let text = "HelloWorld!\r\nPL011\r\n";
    loop {
        print_u8(text, PL011_UART);
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let _ = info;
    loop {
        unsafe { asm!("wfi") };
    }
}
