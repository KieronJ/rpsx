use super::{Instruction, Interconnect};

pub struct CPU {
	regs: [u32; 32],
	regs_load_delay: [u32; 32],

	pc: u32,

	hi: u32,
	lo: u32,

	branch_delay_enabled: bool,
	branch_delay_slot: bool,
	branch_delay_pc: u32,

	load_delay: bool,

	interconnect: Interconnect
}

impl CPU {
	pub fn new(bios: Box<[u8]>) -> CPU {
		CPU {
			regs: [0; 32],
			regs_load_delay: [0; 32],

			pc: 0xbfc0_0000,

			hi: 0,
			lo: 0,

			branch_delay_enabled: false,
			branch_delay_slot: false,
			branch_delay_pc: 0,

			load_delay: false,

			interconnect: Interconnect::new(bios)
		}
	}

	pub fn run(&mut self) {
		print!("{:#08x}: ", self.pc);

		let instruction = Instruction::new(self.load32(self.pc));

		match instruction.opcode() {
			0b001101 => self.op_ori(instruction),
			0b001111 => self.op_lui(instruction),
			0b101011 => self.op_sw(instruction),
			_ => { println!("Unrecognised instruction {:#08x}", instruction.as_bytes()); 
				   panic!("Unrecognised instruction!") }
		}

		//println!("{:?}\n", self.regs);

		self.pc += 4;
	}

	fn op_ori(&mut self, instruction: Instruction) {
		let rt  = instruction.rt();
		let rs  = instruction.rs();
		let imm = instruction.imm();

		println!("ORI ${}, ${}, {:#04x}", rt, rs, imm);

		let d = self.reg(rs as usize) | imm;
		self.set_reg(rt as usize, d);
	}

	fn op_lui(&mut self, instruction: Instruction) {
		let rt  = instruction.rt();
		let imm = instruction.imm();

		println!("LUI ${}, {:#04x}", rt, imm);

		let d = imm << 16;
		self.set_reg(rt as usize, d);
	}

	fn op_sw(&mut self, instruction: Instruction) {
		let rt  = instruction.rt();
		let rs  = instruction.rs();
		let offset = instruction.imm_se();

		println!("SW ${}, {:#04x}(${})", rt, offset, rs);

		let addr = self.reg(rs as usize) + offset;
		let d = self.reg(rt as usize);
		self.store32(addr, d);
	}

	fn load32(&self, address: u32) -> u32 {
		self.interconnect.load32(address)
	}

	fn store32(&mut self, address: u32, data: u32) {
		self.interconnect.store32(address, data);
	}

	fn reg(&self, index: usize) -> u32 {
		self.regs[index]
	}

	fn set_reg(&mut self, index: usize, value: u32) {
		self.regs[index] = value;
		self.regs[0] = 0;
	}

}