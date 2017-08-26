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
pub const TIMER_RANGE: Range = Range(0x1f801100, 0x1f80112c);
pub const DMA_RANGE: Range = Range(0x1f801080, 0x1f801100);
pub const GP0_RANGE: Range = Range(0x1f801810, 0x1f801814);
pub const GP1_RANGE: Range = Range(0x1f801814, 0x1f801818);


pub const REGION_MASK: [u32; 8] =  [0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, // KUSEG
									0x7fffffff, // KSEG0
									0x1fffffff, // KSEG1
									0xffffffff, 0xffffffff]; // KSEG2

#[derive(Default)]
struct InterruptStatus {
	vblank: bool,
	gpu: bool,
	cdrom: bool,
	dma: bool,
	timer: [bool; 3],
	peripherals: bool,
	sio: bool,
	spu: bool,
	pio: bool
}

impl InterruptStatus {
	fn status(&self) -> u32 {
		((self.vblank      as u32) <<  0) |
		((self.gpu         as u32) <<  1) |
		((self.cdrom       as u32) <<  2) |
		((self.dma         as u32) <<  3) |
		((self.timer[0]    as u32) <<  4) |
		((self.timer[1]    as u32) <<  5) |
		((self.timer[2]    as u32) <<  6) |
		((self.peripherals as u32) <<  7) |
		((self.sio         as u32) <<  8) |
		((self.spu         as u32) <<  9) |
		((self.pio         as u32) << 10)
	}

	fn acknowledge(&mut self, value: u32) {
		self.vblank      = (value & 0x1)   != 0;
		self.gpu         = (value & 0x2)   != 0;
		self.cdrom       = (value & 0x4)   != 0;
		self.dma         = (value & 0x8)   != 0;
		self.timer[0]    = (value & 0x10)  != 0;
		self.timer[1]    = (value & 0x20)  != 0;
		self.timer[2]    = (value & 0x40)  != 0;
		self.peripherals = (value & 0x80)  != 0;
		self.sio         = (value & 0x100) != 0;
		self.spu         = (value & 0x200) != 0;
		self.pio         = (value & 0x400) != 0;
	}
}

#[derive(Default)]
struct InterruptMask {
	vblank: bool,
	gpu: bool,
	cdrom: bool,
	dma: bool,
	timer: [bool; 3],
	peripherals: bool,
	sio: bool,
	spu: bool,
	pio: bool
}

impl InterruptMask {
	fn read(&self) -> u32 {
		((self.vblank      as u32) <<  0) |
		((self.gpu         as u32) <<  1) |
		((self.cdrom       as u32) <<  2) |
		((self.dma         as u32) <<  3) |
		((self.timer[0]    as u32) <<  4) |
		((self.timer[1]    as u32) <<  5) |
		((self.timer[2]    as u32) <<  6) |
		((self.peripherals as u32) <<  7) |
		((self.sio         as u32) <<  8) |
		((self.spu         as u32) <<  9) |
		((self.pio         as u32) << 10)
	}

	fn write(&mut self, value: u32) {
		self.vblank      = (value & 0x1)   != 0;
		self.gpu         = (value & 0x2)   != 0;
		self.cdrom       = (value & 0x4)   != 0;
		self.dma         = (value & 0x8)   != 0;
		self.timer[0]    = (value & 0x10)  != 0;
		self.timer[1]    = (value & 0x20)  != 0;
		self.timer[2]    = (value & 0x40)  != 0;
		self.peripherals = (value & 0x80)  != 0;
		self.sio         = (value & 0x100) != 0;
		self.spu         = (value & 0x200) != 0;
		self.pio         = (value & 0x400) != 0;
	}
}

#[derive(Default)]
struct CacheControl {
	cache_enable: bool,
	scratchpad_enable: [bool; 2],
	unknown: u32
}

impl CacheControl {
	fn read(&self) -> u32 {
		self.unknown | (self.scratchpad_enable[0] as u32) <<  3
		             | (self.scratchpad_enable[1] as u32) <<  7
		             | (self.cache_enable         as u32) << 11
	}

	fn write(&mut self, value: u32) {
		self.scratchpad_enable[0] = (value & 0x8)   != 0;
		self.scratchpad_enable[1] = (value & 0x80)  != 0;
		self.cache_enable         = (value & 0x800) != 0;
		self.unknown = value & 0xfffff337;
	}
}

#[derive(Default)]
pub struct Interconnect {
	ram: Box<[u8]>,
	bios: Box<[u8]>,
	interrupt_status: InterruptStatus,
	interrupt_mask: InterruptMask,
	cache_control: CacheControl
}

impl Interconnect {
	pub fn init(&mut self, bios: Box<[u8]>) {
		self.bios = bios;
		self.ram = vec![0; 0x200000].into_boxed_slice()
	}

	pub fn translate_address(&self, address: u32) -> u32 {
		address & REGION_MASK[(address >> 29) as usize]
	}

	pub fn load8(&self, virtual_address: u32) -> u8 {
		let physical_address = self.translate_address(virtual_address);

		match physical_address {
			address if RAM_RANGE.between(address) => self.ram[RAM_RANGE.offset(address)],
			address if BIOS_RANGE.between(address) => self.bios[BIOS_RANGE.offset(address)],
			address if MEM_CONTROL_RANGE.between(address) => { println!("load8 from unimplemented MEM_CONTROL register 0x{:08x}", address); 0 },
			address if RAM_SIZE_RANGE.between(address) => { println!("load8 from unimplemented RAM_SIZE register 0x{:08x}", address); 0 },
			address if SPU_RANGE.between(address) => { println!("load8 from unimplemented SPU register 0x{:08x}", address); 0 },
			address if EXPANSION_1_RANGE.between(address) => { println!("load8 from unimplemented EXPANSION_1 region 0x{:08x}", address); 0xff },
			address if EXPANSION_2_RANGE.between(address) => { println!("load8 from unimplemented EXPANSION_2 region 0x{:08x}", address); 0 },
			address if TIMER_RANGE.between(address) => { println!("load8 from unimplemented TIMER register 0x{:08x}", address); 0 },
			address if DMA_RANGE.between(address) => { println!("load8 from unimplemented DMA register 0x{:08x}", address); 0 },
			address if GP0_RANGE.between(address) => { println!("load8 from unimplemented GPUREAD register 0x{:08x}", address); 0 },
			address if GP1_RANGE.between(address) => { println!("load8 from unimplemented GPUSTAT register 0x{:08x}", address); 0 },
			_ => panic!("load8 from unimplemented range 0x{:08x}", physical_address)
		}
	}

	pub fn load16(&self, virtual_address: u32) -> u16 {
		let physical_address = self.translate_address(virtual_address);

		match physical_address {
			address if RAM_RANGE.between(address) => LittleEndian::read_u16(&self.ram[RAM_RANGE.offset(address)..]),
			address if BIOS_RANGE.between(address) => LittleEndian::read_u16(&self.bios[BIOS_RANGE.offset(address)..]),
			address if MEM_CONTROL_RANGE.between(address) => { println!("load16 from unimplemented MEM_CONTROL register 0x{:08x}", address); 0 },
			address if RAM_SIZE_RANGE.between(address) => { println!("load16 from unimplemented RAM_SIZE register 0x{:08x}", address); 0 },
			address if SPU_RANGE.between(address) => { println!("load16 from unimplemented SPU register 0x{:08x}", address); 0 },
			address if EXPANSION_1_RANGE.between(address) => { println!("load16 from unimplemented EXPANSION_1 region 0x{:08x}", address); 0xffff },
			address if EXPANSION_2_RANGE.between(address) => { println!("load16 from unimplemented EXPANSION_2 region 0x{:08x}", address); 0 },
			address if INTERRUPT_CONTROL_RANGE.between(address) =>
				match INTERRUPT_CONTROL_RANGE.offset(address) {
					0 => self.interrupt_status.status() as u16,
					4 => self.interrupt_mask.read() as u16,
					_ => panic!("load16 from unaligned INTERRUPT_CONTROL register 0x{:08x}", physical_address) //TODO: Handle this properly.
				},
			address if TIMER_RANGE.between(address) => { println!("load16 from unimplemented TIMER register 0x{:08x}", address); 0 },
			address if DMA_RANGE.between(address) => { println!("load16 from unimplemented DMA register 0x{:08x}", address); 0 },
			address if GP0_RANGE.between(address) => { println!("load16 from unimplemented GPUREAD register 0x{:08x}", address); 0 },
			address if GP1_RANGE.between(address) => { println!("load16 from unimplemented GPUSTAT register 0x{:08x}", address); 0 },
			_ => panic!("load16 from unimplemented range 0x{:08x}", physical_address)
		}
	}

	pub fn load32(&self, virtual_address: u32) -> u32 {
		let physical_address = self.translate_address(virtual_address);

		match physical_address {
			address if RAM_RANGE.between(address) => LittleEndian::read_u32(&self.ram[RAM_RANGE.offset(address)..]),
			address if BIOS_RANGE.between(address) => LittleEndian::read_u32(&self.bios[BIOS_RANGE.offset(address)..]),
			address if MEM_CONTROL_RANGE.between(address) => { println!("load32 from unimplemented MEM_CONTROL register 0x{:08x}", address); 0 },
			address if RAM_SIZE_RANGE.between(address) => { println!("load32 from unimplemented RAM_SIZE register 0x{:08x}", address); 0 },
			address if CACHE_CONTROL_RANGE.between(address) => self.cache_control.read(),
			address if SPU_RANGE.between(address) => { println!("load32 from unimplemented SPU register 0x{:08x}", address); 0 },
			address if EXPANSION_1_RANGE.between(address) => { println!("load32 from unimplemented EXPANSION_1 region 0x{:08x}", address); 0xffffffff },
			address if EXPANSION_2_RANGE.between(address) => { println!("load32 from unimplemented EXPANSION_2 region 0x{:08x}", address); 0 },
			address if INTERRUPT_CONTROL_RANGE.between(address) =>
				match INTERRUPT_CONTROL_RANGE.offset(address) {
					0 => self.interrupt_status.status(),
					4 => self.interrupt_mask.read(),
					_ => unreachable!()
				},
			address if TIMER_RANGE.between(address) => { println!("load32 from unimplemented TIMER register 0x{:08x}", address); 0 },
			address if DMA_RANGE.between(address) => { println!("load32 from unimplemented DMA register 0x{:08x}", address); 0 },
			address if GP0_RANGE.between(address) => { println!("load32 from unimplemented GPUREAD register 0x{:08x}", address); 0 },
			address if GP1_RANGE.between(address) => { println!("load32 from unimplemented GPUSTAT register 0x{:08x}", address); 0x10000000 },
			_ => panic!("load32 from unimplemented range 0x{:08x}", physical_address)
		}
	}

	pub fn store8(&mut self, virtual_address: u32, data: u8) {
		let physical_address = self.translate_address(virtual_address);

		match physical_address {
			address if RAM_RANGE.between(address) => self.ram[RAM_RANGE.offset(address)] = data,
			address if BIOS_RANGE.between(address) => panic!("store8 to BIOS range {:#08x}"),
			address if MEM_CONTROL_RANGE.between(address) => println!("store8 to unimplemented MEM_CONTROL register 0x{:08x}", address),
			address if RAM_SIZE_RANGE.between(address) => println!("store8 to unimplemented RAM_SIZE register 0x{:08x}", address),
			address if SPU_RANGE.between(address) => println!("store8 to unimplemented SPU register 0x{:08x}", address),
			address if EXPANSION_1_RANGE.between(address) => println!("store8 to unimplemented EXPANSION_1 range 0x{:08x}", address),
			address if EXPANSION_2_RANGE.between(address) => println!("store8 to unimplemented EXPANSION_2 range 0x{:08x}", address),
			address if TIMER_RANGE.between(address) => println!("store8 to unimplemented TIMER register 0x{:08x}", address),
			address if DMA_RANGE.between(address) => println!("store8 to unimplemented DMA register 0x{:08x}", address),
			address if GP0_RANGE.between(address) => println!("store8 to unimplemented GP0 register 0x{:08x}", address),
			address if GP1_RANGE.between(address) => println!("store8 to unimplemented GP1 register 0x{:08x}", address),
			_ => panic!("store8 to unimplemented range 0x{:08x}", physical_address)
		}
	}

	pub fn store16(&mut self, virtual_address: u32, data: u16) {
		let physical_address = self.translate_address(virtual_address);

		match physical_address {
			address if RAM_RANGE.between(address) => LittleEndian::write_u16(&mut self.ram[(RAM_RANGE.offset(address))..], data),
			address if BIOS_RANGE.between(address) => panic!("store16 to BIOS range {:#08x}"),
			address if MEM_CONTROL_RANGE.between(address) => println!("store16 to unimplemented MEM_CONTROL register 0x{:08x}", address),
			address if RAM_SIZE_RANGE.between(address) => println!("store16 to unimplemented RAM_SIZE register 0x{:08x}", address),
			address if SPU_RANGE.between(address) => println!("store16 to unimplemented SPU register 0x{:08x}", address),
			address if EXPANSION_2_RANGE.between(address) => println!("store16 to unimplemented EXPANSION_1 region 0x{:08x}", address),
			address if EXPANSION_2_RANGE.between(address) => println!("store16 to unimplemented EXPANSION_2 region 0x{:08x}", address),
			address if INTERRUPT_CONTROL_RANGE.between(address) =>
				match INTERRUPT_CONTROL_RANGE.offset(address) {
					0 => self.interrupt_status.acknowledge(data as u32),
					4 => self.interrupt_mask.write(data as u32),
					_ => panic!("store16 to unaligned INTERRUPT_CONTROL register 0x{:08x}", physical_address) //TODO: Handle this properly too.
				},
			address if TIMER_RANGE.between(address) => println!("store16 to unimplemented TIMER register 0x{:08x}", address),
			address if DMA_RANGE.between(address) => println!("store16 to unimplemented DMA register 0x{:08x}", address),
			address if GP0_RANGE.between(address) => println!("store16 to unimplemented GP0 register 0x{:08x}", address),
			address if GP1_RANGE.between(address) => println!("store16 to unimplemented GP1 register 0x{:08x}", address),
			_ => panic!("store16 to unimplemented range 0x{:08x}", physical_address)
		}
	}

	pub fn store32(&mut self, virtual_address: u32, data: u32) {
		let physical_address = self.translate_address(virtual_address);

		match physical_address {
			address if RAM_RANGE.between(address) => LittleEndian::write_u32(&mut self.ram[(RAM_RANGE.offset(address))..], data),
			address if BIOS_RANGE.between(address) => panic!("store32 to BIOS range {:#08x}"),
			address if MEM_CONTROL_RANGE.between(address) => println!("store32 to unimplemented MEM_CONTROL register 0x{:08x}", address),
			address if RAM_SIZE_RANGE.between(address) => println!("store32 to unimplemented RAM_SIZE register 0x{:08x}", address),
			address if CACHE_CONTROL_RANGE.between(address) => self.cache_control.write(data),
			address if SPU_RANGE.between(address) => println!("store32 to unimplemented SPU register 0x{:08x}", address),
			address if EXPANSION_2_RANGE.between(address) => println!("store32 to unimplemented EXPANSION_1 region 0x{:08x}", address),
			address if EXPANSION_2_RANGE.between(address) => println!("store32 to unimplemented EXPANSION_2 region 0x{:08x}", address),
			address if INTERRUPT_CONTROL_RANGE.between(address) =>
				match INTERRUPT_CONTROL_RANGE.offset(address) {
					0 => self.interrupt_status.acknowledge(data),
					4 => self.interrupt_mask.write(data),
					_ => unreachable!()
				},
			address if TIMER_RANGE.between(address) => println!("store32 to unimplemented TIMER register 0x{:08x}", address),
			address if DMA_RANGE.between(address) => println!("store32 to unimplemented DMA register 0x{:08x}", address),
			address if GP0_RANGE.between(address) => println!("store32 to unimplemented GP0 register 0x{:08x}", address),
			address if GP1_RANGE.between(address) => println!("store32 to unimplemented GP1 register 0x{:08x}", address),
			_ => panic!("store32 to unimplemented range 0x{:08x}", physical_address)
		}
	}
}