use tock_registers::interfaces::ReadWriteable;
use tock_registers::interfaces::Readable;
use tock_registers::register_bitfields;
use tock_registers::register_structs;
use tock_registers::registers::{ReadOnly, ReadWrite};

use crate::interfaces::rp1::rp1_info::Rp1Block;
use crate::interfaces::rp1::rp1_info::get_block_address;

register_structs! {
    /// GPIO Peripheral Register Block
    pub IoBank0 {
        (0x000 => pub gpio: [(ReadOnly<u32, GPIO_STATUS::Register>, ReadWrite<u32, GPIO_CTRL::Register>); 28]),
        (0x0E0 => _reserved0e0),
        (0x100 => pub intr: ReadWrite<u32, INT::Register>), // raw interrupts
        (0x104 => pub proc0_inte: ReadWrite<u32, INT::Register>), // interrupt enable for proc0
        (0x108 => pub proc0_intf: ReadWrite<u32, INT::Register>), // interrupt force for proc0
        (0x10C => pub proc0_ints: ReadOnly<u32, INT::Register>), // interrupt status after masking & forcing for proc0
        (0x110 => pub proc1_inte: ReadWrite<u32, INT::Register>), // interrupt enable for proc1
        (0x114 => pub proc1_intf: ReadWrite<u32, INT::Register>), // interrupt force for proc1
        (0x118 => pub proc1_ints: ReadOnly<u32, INT::Register>), // interrupt status after masking & forcing for proc1
        (0x11C => pub pcie_inte: ReadWrite<u32, INT::Register>), // interrupt enable for pcie
        (0x120 => pub pcie_intf: ReadWrite<u32, INT::Register>), // interrupt force for pcie
        (0x124 => pub pcie_ints: ReadOnly<u32, INT::Register>), // interrupt status after masking & forcing for pcie
        (0x128 => @END),
    }
}

register_bitfields!(
    u32,
    pub GPIO_STATUS [
        /// output signal from selected peripheral, before register overide is applied
        OUTFROMPERI OFFSET(8) NUMBITS(1) [],
        /// output signal to pad after register overide is applied
        OUTTOPAD OFFSET(9) NUMBITS(1) [],
        /// output enable from selected peripheral, before register overide is applied
        OEFROMPERI OFFSET(12) NUMBITS(1) [],
        /// output enable to pad after register overide is applied
        OETOPAD OFFSET(13) NUMBITS(1) [],
        /// input signal from pad, goes directly to the selected peripheral without filtering or override
        INISDIRECT OFFSET(16) NUMBITS(1) [],
        /// input signal from pad, before filtering and override are applied
        INFROMPAD OFFSET(17) NUMBITS(1) [],
        /// input signal from pad, after filtering is applied but before override, not valid if inisdirect=1
        INFILTERED OFFSET(18) NUMBITS(1) [],
        /// input signal to peripheral, after filtering and override are applied, not valid if inisdirect=1
        INTOPERI OFFSET(19) NUMBITS(1) [],
        /// Input pin has seen falling edge. Clear with ctrl_irqreset
        EVENT_EDGE_LOW OFFSET(20) NUMBITS(1) [],
        /// Input pin has seen rising edge. Clear with ctrl_irqreset
        EVENT_EDGE_HIGH OFFSET(21) NUMBITS(1) [],
        /// Input pin is Low
        EVENT_LEVEL_LOW OFFSET(22) NUMBITS(1) [],
        /// Input pin is high
        EVENT_LEVEL_HIGH OFFSET(23) NUMBITS(1) [],
        /// Input pin has seen a filtered falling edge. Clear with ctrl_irqreset
        EVENT_F_EDGE_LOW OFFSET(24) NUMBITS(1) [],
        /// Input pin has seen a filtered rising edge. Clear with ctrl_irqreset
        EVENT_F_EDGE_HIGH OFFSET(25) NUMBITS(1) [],
        /// Debounced input pin is low
        EVENT_DB_LEVEL_LOW OFFSET(26) NUMBITS(1) [],
        /// Debounced input pin is high
        EVENT_DB_LEVEL_HIGH OFFSET(27) NUMBITS(1) [],
        /// interrupt to processors, after masking
        IRQCOMBINED OFFSET(28) NUMBITS(1) [],
        /// interrupt to processors, after mask and override is applied
        IRQTOPROC OFFSET(29) NUMBITS(1) []
    ],

    pub GPIO_CTRL [
        FUNCSEL OFFSET(0) NUMBITS(5) [
            NULL = 0x1F // default value
        ],

        /// F_M - Filter/debounce time constant M.
        F_M OFFSET(5) NUMBITS(7) [],

        /// OUTOVER - Output Override.
        OUTOVER OFFSET(12) NUMBITS(2) [
            FromPeripheral = 0, // drive output from peripheral signal selected by funcsel
            InverseFromPeripheral = 1, // drive output from inverse of peripheral signal selected by funcsel
            DriveLow = 2, // drive output low
            DriveHigh = 3 // drive output high
        ],

        /// OEOVER - Output Enable Override.
        OEOVER OFFSET(14) NUMBITS(2) [
            FromPeripheral = 0, // drive output from peripheral signal selected by funcsel
            InverseFromPeripheral = 1, // drive output from inverse of peripheral signal selected by funcsel
            Disable = 2, // disable output
            Enable = 3 // enable output
        ],

        /// INOVER - Input Override.
        INOVER OFFSET(16) NUMBITS(2) [
            DontInvert = 0, // don't invert the peri input
            Invert = 1, // invert the peri input
            DriveLow = 2, // drive peri input low
            DriveHigh = 3 // drive peri input high
        ],

        /// IRQMASK_EDGE_LOW - Masks the edge low interrupt.
        IRQMASK_EDGE_LOW OFFSET(20) NUMBITS(1) [],

        /// IRQMASK_EDGE_HIGH - Masks the edge high interrupt.
        IRQMASK_EDGE_HIGH OFFSET(21) NUMBITS(1) [],

        /// IRQMASK_LEVEL_LOW - Masks the level low interrupt.
        IRQMASK_LEVEL_LOW OFFSET(22) NUMBITS(1) [],

        /// IRQMASK_LEVEL_HIGH - Masks the level high interrupt.
        IRQMASK_LEVEL_HIGH OFFSET(23) NUMBITS(1) [],

        /// IRQMASK_F_EDGE_LOW - Masks the filtered edge low interrupt.
        IRQMASK_F_EDGE_LOW OFFSET(24) NUMBITS(1) [],

        /// IRQMASK_F_EDGE_HIGH - Masks the filtered edge high interrupt.
        IRQMASK_F_EDGE_HIGH OFFSET(25) NUMBITS(1) [],

        /// IRQMASK_DB_LEVEL_LOW - Masks the debounced level low interrupt.
        IRQMASK_DB_LEVEL_LOW OFFSET(26) NUMBITS(1) [],

        /// IRQMASK_DB_LEVEL_HIGH - Masks the debounced level high interrupt.
        IRQMASK_DB_LEVEL_HIGH OFFSET(27) NUMBITS(1) [],

        /// IRQRESET - Reset the interrupt edge detector.
        IRQRESET OFFSET(28) NUMBITS(1) [ // SelfClean(WriteOnly)
            DoNothing = 0,
            Reset = 1 // reset the interrupt edge detector
        ],

        /// IRQOVER - Interrupt Override.
        IRQOVER OFFSET(30) NUMBITS(2) [
            DontInvert = 0, // don't revert the interrupt
            Invert = 1, // invert the interrupt
            DriveLow = 2, // drive the interrupt low
            DriveHigh = 3 // drive the interrupt high
        ],
    ],
    pub INT [
        GPIO0 OFFSET(0)  NUMBITS(2) [],
        GPIO1 OFFSET(1)  NUMBITS(1) [],
        GPIO2 OFFSET(2)  NUMBITS(1) [],
        GPIO3 OFFSET(3)  NUMBITS(1) [],
        GPIO4 OFFSET(4)  NUMBITS(1) [],
        GPIO5 OFFSET(5)  NUMBITS(1) [],
        GPIO6 OFFSET(6)  NUMBITS(1) [],
        GPIO7 OFFSET(7)  NUMBITS(1) [],
        GPIO8 OFFSET(8)  NUMBITS(1) [],
        GPIO9 OFFSET(9)  NUMBITS(1) [],
        GPIO10 OFFSET(10) NUMBITS(1) [],
        GPIO11 OFFSET(11) NUMBITS(1) [],
        GPIO12 OFFSET(12) NUMBITS(1) [],
        GPIO13 OFFSET(13) NUMBITS(1) [],
        GPIO14 OFFSET(14) NUMBITS(1) [],
        GPIO15 OFFSET(15) NUMBITS(1) [],
        GPIO16 OFFSET(16) NUMBITS(1) [],
        GPIO17 OFFSET(17) NUMBITS(1) [],
        GPIO18 OFFSET(18) NUMBITS(1) [],
        GPIO19 OFFSET(19) NUMBITS(1) [],
        GPIO20 OFFSET(20) NUMBITS(1) [],
        GPIO21 OFFSET(21) NUMBITS(1) [],
        GPIO22 OFFSET(22) NUMBITS(1) [],
        GPIO23 OFFSET(23) NUMBITS(1) [],
        GPIO24 OFFSET(24) NUMBITS(1) [],
        GPIO25 OFFSET(25) NUMBITS(1) [],
        GPIO26 OFFSET(26) NUMBITS(1) [],
        GPIO27 OFFSET(27) NUMBITS(1) [],
    ],
);

register_structs! {
    pub PadsBank0 {
        (0x00 => pub voltage_select: ReadWrite<u32, VOLTAGE_SELECT::Register>),
        (0x04 => pub gpio: [ReadWrite<u32, GPIO::Register>; 28]),
        (0x74 => @END),
    }
}

register_bitfields!(
    u32,
    pub VOLTAGE_SELECT [
        VOLTAGE_SELECT OFFSET(0) NUMBITS(1) [], // 0b0: 3.3V 0b1: 1.8V
    ],
    pub GPIO [
        SLEWFAST OFFSET(0) NUMBITS(1) [], // slew rate control, 1 = FAST, 0 = SLOW
        SCHMITT OFFSET(1) NUMBITS(1) [], // enable schmitt trigger (default 0b01)
        PDE OFFSET(2) NUMBITS(1) [], // pull down enable (no default)
        PUE OFFSET(3) NUMBITS(1) [], // pull up enable (no default)
        DRIVE OFFSET(4) NUMBITS(2) [
            STRENGTH_2mA = 0b00,
            STRENGTH_4mA = 0b01,
            STRENGTH_8mA = 0b10,
            STRENGTH_16mA = 0b11,
        ], // drive strength (default 0b01)
        IE OFFSET(6) NUMBITS(1) [], // input enable
        // output disable (has priority over output enable from peripheral)
        OD OFFSET(7) NUMBITS(1) [], // default 0b1
    ],
);

pub struct Rp1GPIO {
    io_bank0: &'static mut IoBank0,
    pads_bank0: &'static mut PadsBank0,
}

impl Rp1GPIO {
    pub fn new() -> Self {
        Self {
            io_bank0: unsafe {
                &mut *(get_block_address(Rp1Block::PadsBank0).address as *mut IoBank0)
            },
            pads_bank0: unsafe {
                &mut *(get_block_address(Rp1Block::PadsBank0).address as *mut PadsBank0)
            },
        }
    }

    pub fn set_func(&self, gpio_num: usize, function: u32) {
        let gpio = self.io_bank0.gpio.get(gpio_num).unwrap();
        gpio.1.modify(GPIO_CTRL::FUNCSEL.val(function));
    }

    pub fn enable_output(&self, gpio_num: usize) {
        let io = self.io_bank0.gpio.get(gpio_num).unwrap();
        io.1.modify(GPIO_CTRL::OUTOVER::FromPeripheral + GPIO_CTRL::OEOVER::FromPeripheral);
        let pads = self.pads_bank0.gpio.get(gpio_num).unwrap();
        pads.modify(GPIO::OD::CLEAR);
    }
}
