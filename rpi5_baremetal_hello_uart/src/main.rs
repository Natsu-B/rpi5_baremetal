#![no_std]
#![no_main]

#[macro_use]
pub mod print;
mod systimer;
use core::{
    arch::{asm, global_asm},
    panic::PanicInfo,
};
use print::_print;
use systimer::SystemTimer;

unsafe extern "C" {
    static mut _BSS_START: usize;
    static mut _BSS_END: usize;
    static mut _STACK_TOP: usize;
}

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

#[unsafe(no_mangle)]
extern "C" fn main() -> ! {
    println!("HelloWorld!\r\nPL011\r\n");
    let mut timer = SystemTimer::new();
    timer.init();
    loop {
        println!("HelloWorld!");
        timer.wait(core::time::Duration::from_secs(1));
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let _ = info;
    loop {
        unsafe { asm!("wfi") };
    }
}
