use super::{Cop0, Instruction, Interconnect};

pub struct CPU {
	regs: [u32; 32],
	load_delay_regs: [u32; 32],

	pc: u32,

	hi: u32,
	lo: u32,

	branch_delay_enabled: bool,
	branch_delay_slot: bool,
	branch_delay_pc: u32,

	load_delay_enabled: bool,
	load_delay_slot: bool,

	cop0: Cop0,

	interconnect: Interconnect
}

impl CPU {
	pub fn new(bios: Box<[u8]>) -> CPU {
		CPU {
			regs: [0; 32],
			load_delay_regs: [0; 32],

			pc: 0xbfc0_0000,

			hi: 0,
			lo: 0,

			branch_delay_enabled: false,
			branch_delay_slot: false,
			branch_delay_pc: 0,

			load_delay_enabled: false,
			load_delay_slot: false,

			cop0: Cop0::new(),

			interconnect: Interconnect::new(bios)
		}
	}

	pub fn run(&mut self) {
		print!("{:#08x}: ", self.pc);

		let instruction = Instruction::new(self.load32(self.pc));

		match instruction.opcode() {
			0b000000 => self.op_special(instruction),
			0b000010 => self.op_j(instruction),
			0b001001 => self.op_addiu(instruction),
			0b001101 => self.op_ori(instruction),
			0b001111 => self.op_lui(instruction),
			0b101011 => self.op_sw(instruction),
			_ => { println!("unrecognised instruction {:#08x}", instruction.as_bytes()); panic!("unrecognised instruction") }
		}

		if self.branch_delay_enabled && !self.branch_delay_slot {
			self.pc = self.branch_delay_pc;
			self.branch_delay_enabled = false;
		} else {
			self.pc += 4;
			self.branch_delay_slot = false;
		}

		if self.load_delay_enabled && self.load_delay_slot {
			self.load_delay_slot = false;
		} else {
			self.regs.copy_from_slice(&self.load_delay_regs);
			self.load_delay_enabled = false;
		}

	}

	fn op_special(&mut self, instruction: Instruction) {
		match instruction.function() {
			0b000000 => self.op_sll(instruction),
			0b100101 => self.op_or(instruction),
			_ => { println!("unrecognised instruction {:#08x}", instruction.as_bytes()); panic!("unrecognised instruction") }		
		}
	}

	fn op_sll(&mut self, instruction: Instruction) {
		let rd  = instruction.rd();
		let rt  = instruction.rt();
		let shift = instruction.shift();

		println!("SLL ${}, ${}, {}", rd, rt, shift);

		let v = self.reg(rt) << shift;
		self.set_reg(rd, v);
	}

	fn op_or(&mut self, instruction: Instruction) {
		let rd = instruction.rd();
		let rt = instruction.rt();
		let rs = instruction.rs();

		println!("OR ${}, ${}, ${}", rd, rs, rt);

		let v = self.reg(rs) | self.reg(rt);

		self.set_reg(rd, v);	
	}

	fn op_j(&mut self, instruction: Instruction) {
		let target = instruction.target();
		let jump_address = (self.pc & 0xf000_0000) | (target << 2);

		println!("J {:#08x}", jump_address);

		self.branch_delay_enabled = true;
		self.branch_delay_slot = true;
		self.branch_delay_pc = jump_address;
	}

	fn op_addiu(&mut self, instruction: Instruction) {
		let rt  = instruction.rt();
		let rs  = instruction.rs();
		let imm = instruction.imm_se();

		println!("ADDIU ${}, ${}, {:#04x}", rt, rs, imm);

		let v = self.reg(rs).wrapping_add(imm);
		self.set_reg(rt, v);

	}

	fn op_ori(&mut self, instruction: Instruction) {
		let rt  = instruction.rt();
		let rs  = instruction.rs();
		let imm = instruction.imm();

		println!("ORI ${}, ${}, {:#04x}", rt, rs, imm);

		let v = self.reg(rs) | imm;
		self.set_reg(rt, v);
	}

	fn op_lui(&mut self, instruction: Instruction) {
		let rt  = instruction.rt();
		let imm = instruction.imm();

		println!("LUI ${}, {:#04x}", rt, imm);

		let v = imm << 16;
		self.set_reg(rt, v);
	}

	fn op_sw(&mut self, instruction: Instruction) {
		let rt  = instruction.rt();
		let rs  = instruction.rs();
		let offset = instruction.imm_se();

		println!("SW ${}, {:#04x}(${})", rt, offset, rs);

		let addr = self.reg(rs).wrapping_add(offset);
		let v = self.reg(rt);
		self.store32(addr, v);
	}

	// fn load8(&self, address: u32) -> u8 {
	// 	(self.interconnect.load8(address) as i8) as u32
	// }

	// fn load16(&self, address: u32) -> u16 {
	// 	(self.interconnect.load16(address) as i16) as u32
	// }

	fn load32(&self, address: u32) -> u32 {
		self.interconnect.load32(address)
	}

	// fn store8(&mut self, address: u32, data: u8) {
	// 	self.interconnect.store8(address, data);
	// }

	// fn store16(&mut self, address: u32, data: u16) {
	// 	self.interconnect.store16(address, data);
	// }

	fn store32(&mut self, address: u32, data: u32) {
		self.interconnect.store32(address, data);
	}

	fn reg(&self, index: usize) -> u32 {
		self.regs[index]
	}

	fn set_reg(&mut self, index: usize, data: u32) {
		self.load_delay_regs[index] = data;
		self.load_delay_regs[0] = 0;
	}

}