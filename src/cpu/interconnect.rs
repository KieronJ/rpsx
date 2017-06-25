use super::Range;
use byteorder::{LittleEndian, ByteOrder};

const BIOS_START: u32 = 0xbfc0_0000;
const BIOS_SIZE: u32  = 0x0008_0000;

const MEM_CONTROL_START: u32 = 0x1f80_1000;
const MEM_CONTROL_SIZE: u32  = 0x0000_0024;

pub struct Interconnect {
	bios: Box<[u8]>,
	bios_range: Range,
	mem_control_range: Range
}

impl Interconnect {
	pub fn new(bios: Box<[u8]>) -> Interconnect {
		Interconnect {
			bios: bios,
			bios_range: Range::new(BIOS_START, BIOS_START + BIOS_SIZE),
			mem_control_range: Range::new(MEM_CONTROL_START, BIOS_START + MEM_CONTROL_SIZE)
		}
	}

	pub fn load32(&self, address: u32) -> u32 {
		if address % 4 != 0 {
			panic!("unaligned load32 address {:#x}", address)
		}


		if self.bios_range.between(address) {
			LittleEndian::read_u32(&self.bios[(address - BIOS_START) as usize..])
		} else if self.mem_control_range.between(address) {
			println!("load32 from unimplemented MEM_CONTROL register {:#x}", address); 0
		} else {
			panic!("load32 from unimplemented range {:#08x}", address)
		}
	}

	pub fn store32(&mut self, address: u32, data: u32) {
		if address % 4 != 0 {
			panic!("unaligned store32 address {:#x}", address)
		}

		if self.bios_range.between(address) {
			panic!("store32 to BIOS range {:#08x}")
		} else if self.mem_control_range.between(address) {
			println!("store32 to unimplemented MEM_CONTROL register {:#08x}", address)
		} else {
			panic!("store32 to unimplemented range {:#08x}", address)
		}
	}
}