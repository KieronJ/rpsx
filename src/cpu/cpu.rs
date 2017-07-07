use super::{Cop0, Instruction, Interconnect};

#[derive(Default)]
pub struct CPU {
	regs: [u32; 32],
	load_delay_regs: [u32; 32],

	pc: u32,

	hi: u32,
	lo: u32,

	branch_delay_enabled: bool,
	branch_delay_slot: bool,
	branch_delay_pc: u32,

	load_delay_slot: bool,

	cop0: Cop0,

	interconnect: Interconnect
}

impl CPU {
	pub fn reset(&mut self, bios: Box<[u8]>) {
		self.pc = 0xbfc00000;
		self.interconnect.reset(bios);
	}

	pub fn run(&mut self) {
		print!("0x{:08x}: ", self.pc);

		let instruction = Instruction::new(self.interconnect.load32(false, self.pc));

		match instruction.opcode() {
			0b000000 => self.op_special(instruction),
			0b000001 => self.op_bxx(instruction),
			0b000010 => self.op_j(instruction),
			0b000011 => self.op_jal(instruction),
			0b000100 => self.op_beq(instruction),
			0b000101 => self.op_bne(instruction),
			0b000110 => self.op_blez(instruction),
			0b000111 => self.op_bgtz(instruction),
			0b001000 => self.op_addi(instruction),
			0b001001 => self.op_addiu(instruction),
			0b001010 => self.op_slti(instruction),
			0b001100 => self.op_andi(instruction),
			0b001101 => self.op_ori(instruction),
			0b001111 => self.op_lui(instruction),
			0b010000 => self.op_cop0(instruction),
			0b100000 => self.op_lb(instruction),
			0b100011 => self.op_lw(instruction),
			0b100100 => self.op_lbu(instruction),
			0b101000 => self.op_sb(instruction),
			0b101001 => self.op_sh(instruction),
			0b101011 => self.op_sw(instruction),
			_ => { println!("unrecognised instruction 0x{:08x}", instruction.as_bytes()); panic!("unrecognised instruction") }
		}

		if self.branch_delay_enabled && !self.branch_delay_slot {
			self.pc = self.branch_delay_pc;
			self.branch_delay_enabled = false;
		} else {
			self.pc = self.pc.wrapping_add(4);
			self.branch_delay_slot = false;
		}

		if self.load_delay_slot {
			self.load_delay_slot = false;
		} else {
			self.regs.copy_from_slice(&self.load_delay_regs);
		}

	}

	fn op_special(&mut self, instruction: Instruction) {
		match instruction.function() {
			0b000000 => self.op_sll(instruction),
			0b000011 => self.op_sra(instruction),
			0b001000 => self.op_jr(instruction),
			0b001001 => self.op_jalr(instruction),
			0b100000 => self.op_add(instruction),
			0b100001 => self.op_addu(instruction),
			0b100011 => self.op_subu(instruction),
			0b100100 => self.op_and(instruction),
			0b100101 => self.op_or(instruction),
			0b101011 => self.op_sltu(instruction),
			_ => { println!("unrecognised instruction 0x{:08x}", instruction.as_bytes()); panic!("unrecognised instruction") }		
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

	fn op_sra(&mut self, instruction: Instruction) {
		let rd    = instruction.rd();
		let rt    = instruction.rt();
		let shift = instruction.shift();

		println!("SRA ${}, ${}, {}", rd, rt, shift);

		let v = (self.reg(rt) as i32) >> shift;
		self.set_reg(rd, v as u32);
	}

	fn op_jr(&mut self, instruction: Instruction) {
		let rs = instruction.rs();
		println!("JR ${}", rs);
		self.branch_delay_enabled = true;
		self.branch_delay_slot = true;
		self.branch_delay_pc = self.reg(rs);
	}

	fn op_jalr(&mut self, instruction: Instruction) {
		let rd = instruction.rd();
		let rs = instruction.rs();
		let pc = self.pc;
		println!("JALR ${}", rs);
		self.branch_delay_enabled = true;
		self.branch_delay_slot = true;
		self.branch_delay_pc = self.reg(rs);
		self.set_reg(rd, pc.wrapping_add(8));
	}

	fn op_add(&mut self, instruction: Instruction) {
		let rd = instruction.rd();
		let rt = instruction.rt();
		let rs = instruction.rs();

		println!("ADD ${}, ${}, ${}", rd, rs, rt);

		let v = (self.reg(rs) as i32).overflowing_add(self.reg(rt) as i32);

		if v.1 {
			panic!("ADD overflow")
		}

		self.set_reg(rd, v.0 as u32);
	}

	fn op_addu(&mut self, instruction: Instruction) {
		let rd = instruction.rd();
		let rt = instruction.rt();
		let rs = instruction.rs();

		println!("ADDU ${}, ${}, ${}", rd, rs, rt);

		let v = self.reg(rs).wrapping_add(self.reg(rt));
		self.set_reg(rd, v);
	}

	fn op_subu(&mut self, instruction: Instruction) {
		let rd = instruction.rd();
		let rt = instruction.rt();
		let rs = instruction.rs();

		println!("SUBU ${}, ${}, ${}", rd, rs, rt);

		let v = self.reg(rs).wrapping_sub(self.reg(rt));
		self.set_reg(rd, v);
	}

	fn op_and(&mut self, instruction: Instruction) {
		let rd = instruction.rd();
		let rt = instruction.rt();
		let rs = instruction.rs();

		println!("AND ${}, ${}, ${}", rd, rs, rt);

		let v = self.reg(rs) & self.reg(rt);
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

	fn op_sltu(&mut self, instruction: Instruction) {
		let rd = instruction.rd();
		let rt = instruction.rt();
		let rs = instruction.rs();

		println!("SLTU ${}, ${}, ${}", rd, rs, rt);

		let v = (self.reg(rs) < self.reg(rt)) as u32;
		self.set_reg(rd, v);
	}

	fn op_bxx(&mut self, instruction: Instruction) {
		let rs = instruction.rs();
		let offset = instruction.imm_se();
		let pc = self.pc;
		let instruction = instruction.as_bytes();

		let bgez = (instruction >> 16) & 0x1;
		let link = (instruction >> 17) & 0xf == 8;

		let mut op = "BLTZ";
		if bgez != 0 {
			op = "BGEZ";
		}

		let mut lnk = "";
		if link {
			self.set_reg(31, pc.wrapping_add(8));
			lnk = "AL";
		}

		let v = self.reg(rs) as i32;
		let test = (v < 0) as u32 ^ bgez;

		println!("{}{} ${}, {}", op, lnk, rs, offset as i16);

		if test != 0 {
			self.branch(offset);
		}
	}

	fn op_j(&mut self, instruction: Instruction) {
		let target = instruction.target();
		let jump_address = (self.pc & 0xf000_0000) | (target << 2);

		println!("J 0x{:08x}", jump_address);

		self.branch_delay_enabled = true;
		self.branch_delay_slot = true;
		self.branch_delay_pc = jump_address;
	}

	fn op_jal(&mut self, instruction: Instruction) {
		let target = instruction.target();
		let pc = self.pc;
		let jump_address = (pc & 0xf000_0000) | (target << 2);

		println!("JAL 0x{:08x}", jump_address);

		self.branch_delay_enabled = true;
		self.branch_delay_slot = true;
		self.branch_delay_pc = jump_address;
		self.set_reg(31, pc.wrapping_add(8));
	}

	fn op_beq(&mut self, instruction: Instruction) {
		let rt = instruction.rt();
		let rs = instruction.rs();
		let offset = instruction.imm_se();

		println!("BEQ ${}, ${}, {}", rs, rt, offset as i16);

		if self.reg(rs) == self.reg(rt) {
			self.branch(offset);
		}
	}

	fn op_bne(&mut self, instruction: Instruction) {
		let rt = instruction.rt();
		let rs = instruction.rs();
		let offset = instruction.imm_se();

		println!("BNE ${}, ${}, {}", rs, rt, offset as i16);

		if self.reg(rs) != self.reg(rt) {
			self.branch(offset);
		}
	}

	fn op_blez(&mut self, instruction: Instruction) {
		let rs = instruction.rs();
		let offset = instruction.imm_se();

		println!("BLEZ ${}, {}", rs, offset as i16);

		if self.reg(rs) as i32 <= 0 {
			self.branch(offset);
		}
	}

	fn op_bgtz(&mut self, instruction: Instruction) {
		let rs = instruction.rs();
		let offset = instruction.imm_se();

		println!("BGTZ ${}, {}", rs, offset as i16);

		if self.reg(rs) as i32 > 0 {
			self.branch(offset);
		}
	}

	fn op_addi(&mut self, instruction: Instruction) {
		let rt  = instruction.rt();
		let rs  = instruction.rs();
		let imm = instruction.imm_se();

		println!("ADDI ${}, ${}, {}", rt, rs, imm as i16);

		let v = (self.reg(rs) as i32).overflowing_add(imm as i32);

		if v.1 {
			panic!("ADDI overflow")
		}

		self.set_reg(rt, v.0 as u32);
	}

	fn op_addiu(&mut self, instruction: Instruction) {
		let rt  = instruction.rt();
		let rs  = instruction.rs();
		let imm = instruction.imm_se();

		println!("ADDIU ${}, ${}, {}", rt, rs, imm as i16);

		let v = self.reg(rs).wrapping_add(imm);
		self.set_reg(rt, v);
	}

	fn op_slti(&mut self, instruction: Instruction) {
		let rt = instruction.rt();
		let rs = instruction.rs();
		let imm = instruction.imm_se() as i32;

		println!("SLTI ${}, ${} {}", rs, rt, imm);

		let v = (self.reg(rs) as i32) < imm;
		self.set_reg(rt, v as u32);
	}

	fn op_andi(&mut self, instruction: Instruction) {
		let rt  = instruction.rt();
		let rs  = instruction.rs();
		let imm = instruction.imm();

		println!("ANDI ${}, ${}, 0x{:04x}", rt, rs, imm);

		let v = self.reg(rs) & imm;
		self.set_reg(rt, v);
	}

	fn op_ori(&mut self, instruction: Instruction) {
		let rt  = instruction.rt();
		let rs  = instruction.rs();
		let imm = instruction.imm();

		println!("ORI ${}, ${}, 0x{:04x}", rt, rs, imm);

		let v = self.reg(rs) | imm;
		self.set_reg(rt, v);
	}

	fn op_lui(&mut self, instruction: Instruction) {
		let rt  = instruction.rt();
		let imm = instruction.imm();

		println!("LUI ${}, 0x{:04x}", rt, imm);

		let v = imm << 16;
		self.set_reg(rt, v);
	}

	fn op_cop0(&mut self, instruction: Instruction) {
		match instruction.rs() {
			0b00000 => self.op_mfc0(instruction),
			0b00100 => self.op_mtc0(instruction),
			_ => { println!("unrecognised cop0 instruction 0x{:08x}", instruction.as_bytes()); panic!("unrecognised cop0 instruction") }	
		}
	}

	fn op_mfc0(&mut self, instruction: Instruction) {
		let cpu_reg = instruction.rt();
		let cop_reg = instruction.rd();

		println!("MTC0 ${}, ${}", cop_reg, cpu_reg);

		let v = self.cop0_reg(cop_reg);
		self.set_reg(cpu_reg, v);
	}

	fn op_mtc0(&mut self, instruction: Instruction) {
		let cpu_reg = instruction.rt();
		let cop_reg = instruction.rd();

		println!("MTC0 ${}, ${}", cop_reg, cpu_reg);

		let v = self.reg(cpu_reg);
		self.set_cop0_reg(cop_reg, v);
	}

	fn op_lb(&mut self, instruction: Instruction) {
		let rt     = instruction.rt();
		let rs     = instruction.rs();
		let offset = instruction.imm_se();

		println!("LB ${}, {}(${})", rt, offset as i16, rs);

		let addr = self.reg(rs).wrapping_add(offset);
		let v = (self.load8(addr) as i8) as u32;
		self.set_reg(rt, v);
	}

	fn op_lw(&mut self, instruction: Instruction) {
		let rt  = instruction.rt();
		let rs  = instruction.rs();
		let offset = instruction.imm_se();

		println!("LW ${}, {}(${})", rt, offset as i16, rs);

		let addr = self.reg(rs).wrapping_add(offset);
		let v = self.load32(addr);
		self.set_reg(rt, v);
	}

	fn op_lbu(&mut self, instruction: Instruction) {
		let rt  = instruction.rt();
		let rs  = instruction.rs();
		let offset = instruction.imm_se();

		println!("LBU ${}, {}(${})", rt, offset as i16, rs);

		let addr = self.reg(rs).wrapping_add(offset);
		let v = self.load8(addr) as u32;
		self.set_reg(rt, v);
	}

	fn op_sb(&mut self, instruction: Instruction) {
		let rt  = instruction.rt();
		let rs  = instruction.rs();
		let offset = instruction.imm_se();

		println!("SB ${}, {}(${})", rt, offset as i16, rs);

		let addr = self.reg(rs).wrapping_add(offset);
		let v = self.reg(rt);
		self.store8(addr, v as u8);
	}

	fn op_sh(&mut self, instruction: Instruction) {
		let rt  = instruction.rt();
		let rs  = instruction.rs();
		let offset = instruction.imm_se();

		println!("SH ${}, {}(${})", rt, offset as i16, rs);

		let addr = self.reg(rs).wrapping_add(offset);
		let v = self.reg(rt);
		self.store16(addr, v as u16);
	}

	fn op_sw(&mut self, instruction: Instruction) {
		let rt  = instruction.rt();
		let rs  = instruction.rs();
		let offset = instruction.imm_se();

		println!("SW ${}, {}(${})", rt, offset as i16, rs);

		let addr = self.reg(rs).wrapping_add(offset);
		let v = self.reg(rt);
		self.store32(addr, v);
	}

	fn load8(&mut self, address: u32) -> u8 {
		self.load_delay_slot = true;
		self.interconnect.load8(self.cop0.isolate_cache(), address)
	}

	fn load32(&mut self, address: u32) -> u32 {
		self.load_delay_slot = true;
		self.interconnect.load32(self.cop0.isolate_cache(), address)
	}

	fn store8(&mut self, address: u32, data: u8) {
		self.interconnect.store8(self.cop0.isolate_cache(), address, data);
	}

	fn store16(&mut self, address: u32, data: u16) {
		self.interconnect.store16(self.cop0.isolate_cache(), address, data);
	}

	fn store32(&mut self, address: u32, data: u32) {
		self.interconnect.store32(self.cop0.isolate_cache(), address, data);
	}

	fn reg(&self, index: usize) -> u32 {
		self.regs[index]
	}

	fn set_reg(&mut self, index: usize, data: u32) {
		self.load_delay_regs[index] = data;
		self.load_delay_regs[0] = 0;
	}

	fn cop0_reg(&mut self, index: usize) -> u32 {
		self.load_delay_slot = true;
		self.cop0.reg(index)
	}

	fn set_cop0_reg(&mut self, index: usize, data: u32) {
		self.cop0.set_reg(index, data);
	}

	fn branch(&mut self, offset: u32) {
		self.branch_delay_enabled = true;
		self.branch_delay_slot = true;
		self.branch_delay_pc = self.pc.wrapping_add(4).wrapping_add(offset << 2);
	}
}