use util::Range;
use byteorder::{LittleEndian, ByteOrder};

pub const RAM_RANGE: Range = Range(0x00000000, 0x00200000);
pub const BIOS_RANGE: Range = Range(0x1fc00000, 0x1fc80000);
pub const MEM_CONTROL_RANGE: Range = Range(0x1f801000, 0x1f801024);
pub const RAM_SIZE_RANGE: Range = Range(0x1f801060, 0x1f801064);
pub const CACHE_CONTROL_RANGE: Range = Range(0xfffe0130, 0xfffe0134);
pub const SPU_RANGE: Range = Range(0x1f801c00 , 0x1f801e80);
pub const EXPANSION_1_RANGE: Range = Range(0x1f000000, 0x1f800000);
pub const EXPANSION_2_RANGE: Range = Range(0x1f802000, 0x1f802042);
pub const INTERRUPT_CONTROL_RANGE: Range = Range(0x1f801070, 0x1f801078);

pub const REGION_MASK: [u32; 8] =  [0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, // KUSEG
									0x7fffffff, // KSEG0
									0x1fffffff, // KSEG1
									0xffffffff, 0xffffffff]; // KSEG2

#[derive(Default)]
pub struct Interconnect {
	bios: Box<[u8]>,
	ram: Box<[u8]>
}

impl Interconnect {
	pub fn reset(&mut self, bios: Box<[u8]>) {
		self.bios = bios;
		self.ram = vec![0; 0x200000].into_boxed_slice()
	}

	pub fn translate_address(&self, address: u32) -> u32 {
		address & REGION_MASK[(address >> 29) as usize]
	}

	pub fn load8(&self, cache_isolated: bool, virtual_address: u32) -> u8 {
		let physical_address = self.translate_address(virtual_address);

		if cache_isolated {
			return 0
		}

		match physical_address {
			address if RAM_RANGE.between(address) => self.ram[RAM_RANGE.offset(address)],
			address if BIOS_RANGE.between(address) => self.bios[BIOS_RANGE.offset(address)],
			address if MEM_CONTROL_RANGE.between(address) => { println!("load8 from unimplemented MEM_CONTROL register 0x{:08x}", address); 0 },
			address if RAM_SIZE_RANGE.between(address) => { println!("load8 from unimplemented RAM_SIZE register 0x{:08x}", address); 0 },
			address if CACHE_CONTROL_RANGE.between(address) => { println!("load8 from unimplemented CACHE_CONTROL register 0x{:08x}", address); 0 },
			address if SPU_RANGE.between(address) => { println!("load8 from unimplemented SPU register 0x{:08x}", address); 0 },
			address if EXPANSION_1_RANGE.between(address) => { println!("load8 from unimplemented EXPANSION_1 region 0x{:08x}", address); 0xff },
			address if EXPANSION_2_RANGE.between(address) => { println!("load8 from unimplemented EXPANSION_2 region 0x{:08x}", address); 0 },
			address if INTERRUPT_CONTROL_RANGE.between(address) => { println!("load8 from unimplemented INTERRUPT_CONTROL register 0x{:08x}", address); 0 },
			_ => panic!("load8 from unimplemented range {:#08x}", physical_address)
		}
	}

	pub fn load32(&self, cache_isolated: bool, virtual_address: u32) -> u32 {
		let physical_address = self.translate_address(virtual_address);

		if physical_address % 4 != 0 {
			panic!("unaligned load32 from address 0x{:08x}", physical_address)
		}

		if cache_isolated {
			return 0
		}

		match physical_address {
			address if RAM_RANGE.between(address) => LittleEndian::read_u32(&self.ram[RAM_RANGE.offset(address)..]),
			address if BIOS_RANGE.between(address) => LittleEndian::read_u32(&self.bios[BIOS_RANGE.offset(address)..]),
			address if MEM_CONTROL_RANGE.between(address) => { println!("load32 from unimplemented MEM_CONTROL register 0x{:08x}", address); 0 },
			address if RAM_SIZE_RANGE.between(address) => { println!("load32 from unimplemented RAM_SIZE register 0x{:08x}", address); 0 },
			address if CACHE_CONTROL_RANGE.between(address) => { println!("load32 from unimplemented CACHE_CONTROL register 0x{:08x}", address); 0 },
			address if SPU_RANGE.between(address) => { println!("load32 from unimplemented SPU register 0x{:08x}", address); 0 },
			address if EXPANSION_1_RANGE.between(address) => { println!("load32 from unimplemented EXPANSION_1 region 0x{:08x}", address); 0xffffffff },
			address if EXPANSION_2_RANGE.between(address) => { println!("load32 from unimplemented EXPANSION_2 region 0x{:08x}", address); 0 },
			address if INTERRUPT_CONTROL_RANGE.between(address) => { println!("load32 from unimplemented INTERRUPT_CONTROL register 0x{:08x}", address); 0 },
			_ => panic!("load32 from unimplemented range {:#08x}", physical_address)
		}
	}

	pub fn store8(&mut self, cache_isolated: bool, virtual_address: u32, data: u8) {
		let physical_address = self.translate_address(virtual_address);

		if cache_isolated {
			return
		}

		match physical_address {
			address if RAM_RANGE.between(address) => self.ram[RAM_RANGE.offset(address)] = data,
			address if BIOS_RANGE.between(address) => panic!("store8 to BIOS range {:#08x}"),
			address if MEM_CONTROL_RANGE.between(address) => println!("store8 to unimplemented MEM_CONTROL register 0x{:08x}", address),
			address if RAM_SIZE_RANGE.between(address) => println!("store8 to unimplemented RAM_SIZE register 0x{:08x}", address),
			address if CACHE_CONTROL_RANGE.between(address) => println!("store8 to unimplemented CACHE_CONTROL register 0x{:08x}", address),
			address if SPU_RANGE.between(address) => println!("store8 to unimplemented SPU register 0x{:08x}", address),
			address if EXPANSION_1_RANGE.between(address) => println!("store8 to unimplemented EXPANSION_1 range 0x{:08x}", address),
			address if EXPANSION_2_RANGE.between(address) => println!("store8 to unimplemented EXPANSION_2 range 0x{:08x}", address),
			address if INTERRUPT_CONTROL_RANGE.between(address) => println!("store8 to unimplemented INTERRUPT_CONTROL register 0x{:08x}", address),
			_ => panic!("store8 to unimplemented range {:#08x}", physical_address)
		}
	}

	pub fn store16(&mut self, cache_isolated: bool, virtual_address: u32, data: u16) {
		let physical_address = self.translate_address(virtual_address);

		if physical_address % 2 != 0 {
			panic!("unaligned store16 to address 0x{:08x}", physical_address)
		}

		if cache_isolated {
			return
		}

		match physical_address {
			address if RAM_RANGE.between(address) => LittleEndian::write_u16(&mut self.ram[(RAM_RANGE.offset(address)) as usize..], data),
			address if BIOS_RANGE.between(address) => panic!("store16 to BIOS range {:#08x}"),
			address if MEM_CONTROL_RANGE.between(address) => println!("store16 to unimplemented MEM_CONTROL register 0x{:08x}", address),
			address if RAM_SIZE_RANGE.between(address) => println!("store16 to unimplemented RAM_SIZE register 0x{:08x}", address),
			address if CACHE_CONTROL_RANGE.between(address) => println!("store16 to unimplemented CACHE_CONTROL register 0x{:08x}", address),
			address if SPU_RANGE.between(address) => println!("store16 to unimplemented SPU register 0x{:08x}", address),
			address if EXPANSION_2_RANGE.between(address) => println!("store16 to unimplemented EXPANSION_1 region 0x{:08x}", address),
			address if EXPANSION_2_RANGE.between(address) => println!("store16 to unimplemented EXPANSION_2 region 0x{:08x}", address),
			address if INTERRUPT_CONTROL_RANGE.between(address) => println!("store16 to unimplemented INTERRUPT_CONTROL register 0x{:08x}", address),
			_ => panic!("store16 to unimplemented range {:#08x}", physical_address)
		}
	}

	pub fn store32(&mut self, cache_isolated: bool, virtual_address: u32, data: u32) {
		let physical_address = self.translate_address(virtual_address);

		if physical_address % 4 != 0 {
			panic!("unaligned store32 to address 0x{:08x}", physical_address)
		}

		if cache_isolated {
			return
		}

		match physical_address {
			address if RAM_RANGE.between(address) => LittleEndian::write_u32(&mut self.ram[(RAM_RANGE.offset(address)) as usize..], data),
			address if BIOS_RANGE.between(address) => panic!("store32 to BIOS range {:#08x}"),
			address if MEM_CONTROL_RANGE.between(address) => println!("store32 to unimplemented MEM_CONTROL register 0x{:08x}", address),
			address if RAM_SIZE_RANGE.between(address) => println!("store32 to unimplemented RAM_SIZE register 0x{:08x}", address),
			address if CACHE_CONTROL_RANGE.between(address) => println!("store32 to unimplemented CACHE_CONTROL register 0x{:08x}", address),
			address if SPU_RANGE.between(address) => println!("store32 to unimplemented SPU register 0x{:08x}", address),
			address if EXPANSION_2_RANGE.between(address) => println!("store32 to unimplemented EXPANSION_1 region 0x{:08x}", address),
			address if EXPANSION_2_RANGE.between(address) => println!("store32 to unimplemented EXPANSION_2 region 0x{:08x}", address),
			address if INTERRUPT_CONTROL_RANGE.between(address) => println!("store32 to unimplemented INTERRUPT_CONTROL register 0x{:08x}", address),
			_ => panic!("store32 to unimplemented range {:#08x}", physical_address)
		}
	}
}