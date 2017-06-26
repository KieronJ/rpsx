use super::Range;
use byteorder::{LittleEndian, ByteOrder};

pub const BIOS_RANGE: Range = Range(0xbfc00000, 0xbfc80000);
pub const MEM_CONTROL_RANGE: Range = Range(0x1f801000, 0x1f801024);
pub const RAM_SIZE_RANGE: Range = Range(0x1f801060, 0x1f801064);
pub const CACHE_CONTROL_RANGE: Range = Range(0xfffe0130, 0xfffe0134);

pub struct Interconnect {
	bios: Box<[u8]>,
}

impl Interconnect {
	pub fn new(bios: Box<[u8]>) -> Interconnect {
		Interconnect {
			bios: bios,
		}
	}

	pub fn load32(&self, address: u32) -> u32 {
		if address % 4 != 0 {
			panic!("unaligned load32 from address {:#x}", address)
		}


		match address {
			address if BIOS_RANGE.between(address) => LittleEndian::read_u32(&self.bios[(BIOS_RANGE.offset(address)) as usize..]),
			address if MEM_CONTROL_RANGE.between(address) => { println!("load32 from unimplemented MEM_CONTROL register {:#x}", address); 0 },
			address if RAM_SIZE_RANGE.between(address) => { println!("load32 from unimplemented RAM_SIZE register {:#x}", address); 0 },
			address if CACHE_CONTROL_RANGE.between(address) => { println!("load32 from unimplemented CACHE_CONTROL register {:#x}", address); 0 }
			_ => panic!("load32 from unimplemented range {:#08x}", address)
		}
	}

	pub fn store32(&mut self, address: u32, data: u32) {
		if address % 4 != 0 {
			panic!("unaligned store32 to address {:#x}", address)
		}

		match address {
			address if BIOS_RANGE.between(address) => panic!("store32 to BIOS range {:#08x}"),
			address if MEM_CONTROL_RANGE.between(address) => println!("store32 to unimplemented MEM_CONTROL register {:#x}", address),
			address if RAM_SIZE_RANGE.between(address) => println!("store32 to unimplemented RAM_SIZE register {:#x}", address),
			address if CACHE_CONTROL_RANGE.between(address) => println!("store32 to unimplemented CACHE_CONTROL register {:#x}", address),
			_ => panic!("store32 to unimplemented range {:#08x}", address)
		}
	}
}