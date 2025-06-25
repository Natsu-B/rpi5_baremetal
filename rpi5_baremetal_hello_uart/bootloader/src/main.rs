#![feature(once_cell_get_mut)]
#![no_std]
#![no_main]
#![recursion_limit = "256"]

#[macro_use]
pub mod print;
pub mod interfaces;
pub mod spinlock;
mod systimer;
use crate::interfaces::{
    pl011::{Pl011Uart, UartNum},
    rp1::{rp1_gpio::Rp1GPIO, rp1_info::get_block_address},
};
use core::{
    arch::{asm, global_asm},
    cell::OnceCell,
    ops::ControlFlow,
    panic::PanicInfo,
};
use dtb::{self, DtbParser};
use systimer::SystemTimer;

unsafe extern "C" {
    static mut _BSS_START: usize;
    static mut _BSS_END: usize;
    static mut _STACK_TOP: usize;
}

const PL011_UART_ADDR: *const u32 = 0x10_7D00_1000 as *const u32;
const RP1_OFFSET_ADDR: usize = 0x1f_0000_0000;
const RP1_BASE_ADDR: u32 = 0x4000_0000;
const RP1_UART0_ADDR: u32 =
    get_block_address(crate::interfaces::rp1::rp1_info::Rp1Block::Uart0).address;
const UART0_ADDR: *const u32 =
    (RP1_OFFSET_ADDR + (RP1_UART0_ADDR - RP1_BASE_ADDR) as usize) as *const u32;

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
    let dtb = DtbParser::init(0x2000_0000).unwrap();
    let pl011_debug_uart_addr = OnceCell::new();
    dtb.find_node(None, Some("arm,pl011"), &mut |(address, _size)| {
        pl011_debug_uart_addr.set(address).unwrap();
        ControlFlow::Continue(())
    })
    .unwrap();
    let debug_uart = Pl011Uart::new(*pl011_debug_uart_addr.get().unwrap() as *const u32);
    debug_uart.init(UartNum::Debug, 115200);
    debug_uart.write("debug uart starting...\r\n");
    // check if the PL011_OFFSET_ADDR is correct
    let chip_id = unsafe { *PL011_UART_ADDR };
    if chip_id == 0x2000_1927 {
        debug_uart.write("PL011_OFFSET_ADDR is correct\r\n");
    } else {
        debug_uart.write("PL011_OFFSET_ADDR is incorrect\r\n");
    }
    //DEBUG_UART.call_once(|| Mutex::new(debug_uart));
    //println!("{chip_id}");
    // if DEBUG_UART.set(debug_uart).is_err() {
    //     println!("failed to set debug uart");
    // }
    //println!("HelloWorld!\r\nPL011\r\n");
    // enable GPIO14, 15
    let gpio = Rp1GPIO::new();
    gpio.set_func(14, 4);
    gpio.set_func(15, 4);
    gpio.enable_output(14);
    gpio.enable_output(15);
    gpio.enable_output(18);
    // setup rp1 uart
    let rp1_uart = Pl011Uart::new(UART0_ADDR);
    rp1_uart.init(UartNum::Rp1 { device_num: 0 }, 115200);
    rp1_uart.write("rp1 uart starting...\r\n");
    // if RP1_UART.set(rp1_uart).is_err() {
    //     println!("failed to set rp1 uart");
    // }
    // init timer
    let mut timer = SystemTimer::new();
    timer.init();
    loop {
        gpio.gpio_enable(18);
        //println!("HelloWorld!\r\n");
        rp1_uart.write("Hello from RP1\r\n");
        timer.wait(core::time::Duration::from_secs(1));
        gpio.gpio_disable(18);
        //println!("HelloWorld!\r\n");
        rp1_uart.write("Hello from RP1\r\n");
        timer.wait(core::time::Duration::from_secs(1));
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let debug_uart = Pl011Uart::new(PL011_UART_ADDR);
    debug_uart.init(UartNum::Debug, 115200);
    debug_uart.write("core 0 panicked!!!\r\n");
    //println!("panicked: {}", info);
    loop {
        unsafe { asm!("wfi") };
    }
}
