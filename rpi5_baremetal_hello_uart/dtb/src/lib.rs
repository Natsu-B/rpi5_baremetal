#![cfg_attr(not(test), no_std)]

use crate::big_endian::{CharStringIter, Dtb, FdtProperty};
use core::{
    ffi::{CStr, c_char, c_str},
    slice, usize,
};
#[cfg(test)]
use std::{collections::HashMap, string::String};

pub struct DtbParser {
    dtb_header: Dtb,
}

#[cfg(alloc)]
enum DeviceProperty {
    String(&'static str),
    StringList(Vec<&'static str>),
    U32(u32),
    Cells(Vec<u32>),
    Empty,
}

#[cfg(alloc)]
struct DeviceNode<'a> {
    name: &'static [u8],
    full_name: &'static [u8],
    properties: HashMap<&'static [u8], DeviceProperty>,
    parent: Option<NonNull<&'a DeviceNode<'a>>>,
    child: Option<Vec<NonNull<&'a DeviceNode<'a>>>>,
}

struct SimpleDeviceNode {
    parent_address_cells: Option<u32>,
    parent_size_cells: Option<u32>,
    address_cells: u32,
    size_cells: u32,
    reg: Option<usize>,
    ranges: Option<usize>,
}

impl SimpleDeviceNode {
    const ADDRESS_CELLS: &str = "#address-cells";
    const SIZE_CELLS: &str = "#size-cells";
    const PROP_COMPATIBLE: &str = "compatible";
    const PROP_DEVICE_NAME: &str = "device_type";
    const PROP_REG: &str = "reg";
    const PROP_RANGES: &str = "ranges";

    pub fn new(parent: Option<&SimpleDeviceNode>) -> Self {
        Self {
            parent_address_cells: parent.map(|e| e.address_cells),
            parent_size_cells: parent.map(|e| e.size_cells),
            address_cells: 2,
            size_cells: 1,
            reg: None,
            ranges: None,
        }
    }

    // address is assumed to point to the FDT_PROP token
    pub fn parse_prop(
        &mut self,
        parser: &DtbParser,
        address: &mut usize,
        device_name: Option<&str>,
        compatible_name: Option<&str>,
    ) -> Result<bool, &'static str> {
        *address += DtbParser::SIZEOF_FDT_TOKEN;
        let property = unsafe { &*(*address as *const FdtProperty) };
        #[cfg(test)]
        assert_eq!(size_of::<FdtProperty>(), 8);
        *address += size_of::<FdtProperty>();
        let name = Dtb::read_char_str(
            parser.dtb_header.get_string_start_address() + property.get_name_offset() as usize,
        )?;
        pr_debug!(
            "FDT_PROP offset: {}, str: {}",
            property.get_name_offset(),
            name
        );
        let mut result = false;
        if let Some(s) = match name {
            Self::ADDRESS_CELLS => {
                self.address_cells = Dtb::read_u32_from_ptr(*address);
                pr_debug!("address_cells: {}", self.address_cells);
                Some(size_of::<u32>())
            }
            Self::SIZE_CELLS => {
                self.size_cells = Dtb::read_u32_from_ptr(*address);
                pr_debug!("size_cells: {}", self.size_cells);
                Some(size_of::<u32>())
            }
            Self::PROP_COMPATIBLE => {
                if let Some(compatible_name) = compatible_name {
                    for str in CharStringIter::new(*address, property.get_property_len()) {
                        if compatible_name == str? {
                            result = true;
                        }
                    }
                }
                None
            }
            Self::PROP_DEVICE_NAME => {
                if let Some(device_name) = device_name
                    && device_name == Dtb::read_char_str(*address)?
                {
                    result = true;
                }
                None
            }
            Self::PROP_REG => {
                self.reg = Some(*address);
                Some(
                    self.parent_address_cells
                        .ok_or("'reg' property should not be located at the root node")?
                        as usize
                        * size_of::<u32>()
                        + self
                            .parent_size_cells
                            .ok_or("'reg' property should not be located at the root node")?
                            as usize
                            * size_of::<u32>(),
                )
            }
            Self::PROP_RANGES => {
                self.ranges = Some(*address);
                if property.get_property_len() != 0 {
                    pr_debug!(
                        "parent address: {}, child address: {}, child_size: {}",
                        self.parent_address_cells.unwrap(),
                        self.address_cells,
                        self.size_cells
                    );
                    None
                } else {
                    Some(0)
                }
            }
            _ => None,
        } {
            if s > property.get_property_len() as usize {
                pr_debug!(
                    "invalid size!!! expected: {} actually: {}",
                    property.get_property_len(),
                    s
                );
                return Err("invalid size");
            }
        }
        *address += property
            .get_property_len()
            .next_multiple_of(DtbParser::ALIGNMENT) as usize;
        Ok(result)
    }

    fn read_reg_internal(&self) -> Result<Option<(usize, usize)>, &'static str> {
        if let Some(reg) = self.reg {
            let address_cells = self.parent_address_cells.unwrap();
            let size_cells = self.parent_size_cells.unwrap();
            if address_cells as usize > (size_of::<usize>() / size_of::<u32>())
                || size_cells as usize > (size_of::<usize>() / size_of::<u32>())
            {
                return Err("address or size cells overflow usize");
            }
            let address = Dtb::read_regs(reg, address_cells)?;
            let len = Dtb::read_regs(reg + address.1, size_cells)?;
            pr_debug!("reg: address: {:#x}, size: {:#x}", address.0, len.0);
            return Ok(Some((address.0, len.0)));
        }
        Ok(None)
    }

    fn read_round_internal(&self) -> Result<Option<(usize, usize, usize)>, &'static str> {
        if let Some(reg) = self.ranges {
            let child_address = Dtb::read_regs(reg, self.address_cells)?;
            #[cfg(test)]
            assert_eq!(
                child_address.1,
                self.address_cells as usize * size_of::<u32>()
            );
            let parent_address =
                Dtb::read_regs(reg + child_address.1, self.parent_address_cells.unwrap())?;
            #[cfg(test)]
            assert_eq!(
                parent_address.1,
                self.parent_address_cells.unwrap() as usize * size_of::<u32>()
            );
            let parent_len =
                Dtb::read_regs(reg + child_address.1 + parent_address.1, self.size_cells)?;
            #[cfg(test)]
            assert_eq!(parent_len.1, self.size_cells as usize * size_of::<u32>());
            return Ok(Some((child_address.0, parent_address.0, parent_len.0)));
        }
        Ok(None)
    }

    pub fn calculate_address(
        &self,
        address: Option<&(usize, usize)>,
    ) -> Result<(usize, usize), &'static str> {
        if let Some(s) = address {
            pr_debug!("tmp{:#x}", s.0 + s.1);
            if self.ranges.is_some() {
                let parent = self.read_round_internal()?.unwrap(); // child address, parent address, child length
                pr_debug!("{:#x} ", parent.0 + parent.2);
                if parent.0 + parent.2 < s.0 + s.1 {
                    return Err("ranges size overflow");
                }
                pr_debug!(
                    "child_address: {:#x}, parent_address: {:#x}, child_len: {:#x}",
                    parent.0,
                    parent.1,
                    parent.2
                );
                Ok((s.0 - parent.0 + parent.1, s.1))
            } else if self.reg.is_some() {
                let parent = self.read_reg_internal()?.unwrap();
                if parent.1 < s.0 + s.1 {
                    return Err("reg size overflow");
                }
                Ok((parent.0 + s.0, s.1))
            } else {
                Ok(*s)
            }
        } else {
            Ok(self.read_reg_internal()?.unwrap()) // unwrap is unreachable
        }
    }
}

impl DtbParser {
    const SIZEOF_FDT_TOKEN: usize = 4;
    const ALIGNMENT: u32 = 4;
    const FDT_BEGIN_NODE: [u8; Self::SIZEOF_FDT_TOKEN] = [0x00, 0x00, 0x00, 0x01];
    const FDT_END_NODE: [u8; Self::SIZEOF_FDT_TOKEN] = [0x00, 0x00, 0x00, 0x02];
    const FDT_PROP: [u8; Self::SIZEOF_FDT_TOKEN] = [0x00, 0x00, 0x00, 0x03];
    const FDT_NOP: [u8; Self::SIZEOF_FDT_TOKEN] = [0x00, 0x00, 0x00, 0x04];
    const FDT_END: [u8; Self::SIZEOF_FDT_TOKEN] = [0x00, 0x00, 0x00, 0x05];
    pub fn init(dtb_address: usize) -> Result<Self, &'static str> {
        let dtb = Dtb::new(dtb_address)?;
        let parser = Self { dtb_header: dtb };
        return Ok(parser);
    }
    pub fn skip_nop(&self, address: &mut usize) {
        while Self::get_types(address) == Self::FDT_NOP {
            *address += Self::SIZEOF_FDT_TOKEN;
        }
    }
    pub fn get_types(address: &usize) -> [u8; 4] {
        unsafe { *(*address as *const [u8; Self::SIZEOF_FDT_TOKEN]) }
    }

    fn find_node_recursive(
        &self,
        pointer: &mut usize,
        device_name: Option<&str>,
        compatible_name: Option<&str>,
        node_info: Option<&SimpleDeviceNode>,
    ) -> Result<Option<(usize, usize)>, &'static str> {
        let mut prop = SimpleDeviceNode::new(node_info);
        if Self::get_types(&pointer) != Self::FDT_BEGIN_NODE {
            return Err("pointer is not begin node");
        }
        *pointer += Self::SIZEOF_FDT_TOKEN;
        let node_name = Dtb::read_char_str(*pointer)?;
        pr_debug!("FDT_BEGIN_NODE node_name: {}", node_name);
        *pointer +=
            (node_name.len() + 1/* null terminator */).next_multiple_of(Self::ALIGNMENT as usize);
        let mut find = false;
        loop {
            pr_debug!("FDT types: {:?}", Self::get_types(pointer));
            match Self::get_types(pointer) {
                Self::FDT_NOP => *pointer += Self::SIZEOF_FDT_TOKEN,
                Self::FDT_END_NODE => {
                    *pointer += Self::SIZEOF_FDT_TOKEN;
                    if find {
                        return Ok(Some(prop.calculate_address(None)?));
                    }
                    return Ok(None);
                }
                Self::FDT_BEGIN_NODE => {
                    if let Some(s) = self.find_node_recursive(
                        pointer,
                        device_name,
                        compatible_name,
                        Some(&prop),
                    )? {
                        return Ok(Some(prop.calculate_address(Some(&s))?));
                    }
                }
                Self::FDT_PROP => {
                    if prop.parse_prop(self, pointer, device_name, compatible_name)? {
                        find = true;
                    }
                }
                _ => {
                    pr_debug!(
                        "find an unknown or unexpected token: {:?}, address: 0x{:#?}",
                        Self::get_types(pointer),
                        *pointer - self.dtb_header.get_struct_start_address()
                    );
                    return Err("find an unknown or unexpected token while parsing the DTB");
                }
            }
        }
    }
    pub fn find_node(
        &self,
        device_name: Option<&str>,
        compatible_name: Option<&str>,
    ) -> Result<Option<(usize, usize)>, &'static str> {
        if device_name.is_some() && compatible_name.is_some()
            || device_name.is_none() && compatible_name.is_none()
        {
            return Err("device name and compatible name cannot be searched for at the same time");
        }
        let mut pointer = self.dtb_header.get_struct_start_address();
        return self.find_node_recursive(&mut pointer, device_name, compatible_name, None);
    }
    #[cfg(alloc)]
    fn parse(&self) -> DeviceNode {
        unimplemented!();
    }
}

mod big_endian {
    // big endianで読み出さないといけないので、modで囲って関連関数を使わないと呼び出せないように
    use super::*;
    use crate::pr_debug;
    #[repr(C)]
    struct FtdHeader {
        magic: u32,             // should contain 0xd00dfeed (big-endian)
        total_size: u32,        // total size of the DTB in bytes
        off_dt_struct: u32,     // structure block offset (byte)
        off_dt_strings: u32,    // strings block offset (byte)
        off_mem_rsvmap: u32,    // memory reservation block offset (byte)
        version: u32,           // version of the device tree data structure
        last_comp_version: u32, // lowest version of the device tree data structure (backward compatible)
        boot_cpuid_phys: u32,   // physical CPU id
        size_dt_strings: u32,   // strings block section
        size_dt_struct: u32,    // structure block section
    }
    #[repr(C)]
    pub struct FdtProperty {
        property_len: u32,
        name_offset: u32,
    }

    impl FdtProperty {
        pub fn get_property_len(&self) -> u32 {
            u32::from_be(self.property_len)
        }
        pub fn get_name_offset(&self) -> u32 {
            u32::from_be(self.name_offset)
        }
    }

    #[repr(C)]
    pub struct FdtReserveEntry {
        address: u64,
        size: u64,
    }

    impl FdtReserveEntry {
        pub fn get_address(&self) -> u64 {
            u64::from_be(self.address)
        }
        pub fn get_size(&self) -> u64 {
            u64::from_be(self.size)
        }
    }

    pub struct Dtb {
        address: &'static FtdHeader,
    }

    impl Dtb {
        const DTB_VERSION: u32 = 17;
        const DTB_HEADER_MAGIC: u32 = 0xd00d_feed;
        pub fn new(address: usize) -> Result<Dtb, &'static str> {
            let ftb = Self {
                address: unsafe { &*(address as *const FtdHeader) },
            };
            let header = ftb.address;
            pr_debug!("dtb version: {}", u32::from_be(header.last_comp_version));
            if u32::from_be(header.magic) != Self::DTB_HEADER_MAGIC {
                return Err("invalid magic");
            }
            if u32::from_be(header.last_comp_version) > Self::DTB_VERSION {
                return Err("this dtb is not compatible with the version 17");
            }
            Ok(ftb)
        }
        pub fn get_struct_start_address(&self) -> usize {
            self.address as *const _ as usize + u32::from_be(self.address.off_dt_struct) as usize
        }
        pub fn get_struct_end_address(&self) -> usize {
            self.get_struct_start_address() + u32::from_be(self.address.size_dt_struct) as usize
        }
        pub fn get_string_start_address(&self) -> usize {
            self.address as *const _ as usize + u32::from_be(self.address.off_dt_strings) as usize
        }
        pub fn get_string_end_address(&self) -> usize {
            self.get_string_start_address() + u32::from_be(self.address.size_dt_strings) as usize
        }
        pub fn get_memory_reservation_start_address(&self) -> usize {
            self.address as *const _ as usize + u32::from_be(self.address.off_mem_rsvmap) as usize
        }
        pub fn read_u32_from_ptr(address: usize) -> u32 {
            u32::from_be(unsafe { *(address as *const u32) })
        }
        // reads null-terminated strings and covert to &str
        pub fn read_char_str(address: usize) -> Result<&'static str, &'static str> {
            let str = unsafe { CStr::from_ptr(address as *const c_char) };
            Ok(str
                .to_str()
                .map_err(|_| "failed to convert &Cstr to &str")?)
        }
        // read 'reg' property value
        pub fn read_regs(address: usize, size: u32) -> Result<(usize, usize), &'static str> {
            let mut address_result = 0;
            let mut address_consumed = 0;
            pr_debug!("read_reg: {}", size);
            for _ in 0..size {
                address_result <<= 32;
                address_result +=
                    u32::from_be(unsafe { *((address + address_consumed) as *const u32) }) as usize;
                address_consumed += size_of::<u32>();
            }
            Ok((address_result, address_consumed))
        }
    }
    // an iterator over a list of null terminated strings within a property
    pub struct CharStringIter {
        pointer: usize,
        remain_size: u32,
    }
    impl CharStringIter {
        pub fn new(pointer: usize, remain_size: u32) -> Self {
            Self {
                pointer,
                remain_size,
            }
        }
        fn next_internal(&mut self) -> Result<&'static str, &'static str> {
            let str = Dtb::read_char_str(self.pointer);
            if let Ok(s) = str {
                let str_size = s.len() + 1 /* null terminator */;
                self.pointer += str_size;
                self.remain_size = self
                    .remain_size
                    .checked_sub(str_size.try_into().map_err(|_| "over flow")?)
                    .ok_or("too small")?;
            }
            str
        }
    }
    impl Iterator for CharStringIter {
        type Item = Result<&'static str, &'static str>;
        fn next(&mut self) -> Option<Self::Item> {
            if self.remain_size == 0 {
                return None;
            }
            Some(self.next_internal())
        }
    }
}
#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::*;
    const PL011_DEBUG_UART_ADDRESS: usize = 0x10_7D00_1000;
    const PL011_DEBUG_UART_SIZE: usize = 0x200;
    const MEMORY_ADDRESS: usize = 0x0;
    const MEMORY_SIZE: usize = 0x2800_0000;
    #[test]
    fn it_works() {
        let test_data = std::fs::read("test/test.dtb").expect("failed to load dtb files");
        pr_debug!(
            "load test file. Size: {} address: 0x{:#?}",
            test_data.len(),
            test_data.as_ptr()
        );
        let mut test_data_addr = test_data.as_ptr() as usize;
        let parser = DtbParser::init(test_data_addr).unwrap();
        let pl011 = parser.find_node(None, Some("arm,pl011")).unwrap().unwrap();
        pr_debug!("PL011: address:{:#x}, size: {:#x}", pl011.0, pl011.1);
        assert_eq!(PL011_DEBUG_UART_ADDRESS, pl011.0);
        assert_eq!(PL011_DEBUG_UART_SIZE, pl011.1);
        let memory = parser.find_node(Some("memory"), None).unwrap().unwrap();
        pr_debug!("memory: address{:#x}, size: {:#x}", memory.0, memory.1);
        assert_eq!(MEMORY_ADDRESS, memory.0);
        assert_eq!(MEMORY_SIZE, memory.1);
        assert_eq!(
            parser.find_node(None, Some("None")).unwrap().is_none(),
            true
        );
    }
}

#[cfg(test)]
#[macro_export]
macro_rules! pr_debug {
    ($fmt:expr) => (println!($fmt));
    ($fmt:expr, $($arg:tt)*) => (println!($fmt, $($arg)*));
}

#[cfg(not(test))]
#[macro_export]
macro_rules! pr_debug {
    ($fmt:expr) => {};
    ($fmt:expr, $($arg:tt)*) => {};
}
