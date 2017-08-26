use super::{ Cop0, Instruction, Interconnect };

use cpu::mips_instruction::*;

#[derive(Debug)]
pub enum Exception {
	ADDRLOAD = 4,
	ADDRSTORE = 5,
	SYSCALL = 8,
	BREAKPOINT = 9,
	RESERVED = 10,
	COPROCESSOR = 11,
	OVERFLOW = 12,
}

#[derive(Default)]
pub struct CPU {
	pub regs: [u32; 32],
	pub load_delay_regs: [u32; 32],

	pub pc: u32,

	pub hi: u32,
	pub lo: u32,

	pub branch_delay_enabled: bool,
	pub branch_delay_slot: bool,
	pub branch_delay_pc: u32,

	pub cop0: Cop0,

	pub interconnect: Interconnect
}

impl CPU {
	pub fn init(&mut self, bios: Box<[u8]>) {
		self.interconnect.init(bios);
		self.reset();
	}

	pub fn reset(&mut self) {
		self.cop0.reset(self.pc);
		self.pc = 0xbfc00000;
	}

	pub fn run(&mut self) {
		if self.pc % 4 != 0 {
			self.enter_exception(Exception::ADDRLOAD)
		}

		//print!("0x{:08x}: ", self.pc);

		self.branch_delay_slot = self.branch_delay_enabled;
		self.branch_delay_enabled = false;

		self.regs = self.load_delay_regs;

		self.decode_and_execute();

		if self.branch_delay_slot { 
			self.pc = self.branch_delay_pc; 
		}  else { 
			self.pc = self.pc.wrapping_add(4); 
		}
	}

	fn decode_and_execute(&mut self) {
		let instruction = Instruction::new(self.interconnect.load32(self.pc));

		match instruction.opcode() {
			0b000000 => match instruction.function() {
				0b000000 => op_sll(self, instruction),
				0b000010 => op_srl(self, instruction),
				0b000011 => op_sra(self, instruction),
				0b000100 => op_sllv(self, instruction),
				0b000110 => op_srlv(self, instruction),
				0b000111 => op_srav(self, instruction),
				0b001000 => op_jr(self, instruction),
				0b001001 => op_jalr(self, instruction),
				0b001100 => op_syscall(self),
				0b001101 => op_break(self),
				0b010000 => op_mfhi(self, instruction),
				0b010001 => op_mthi(self, instruction),
				0b010010 => op_mflo(self, instruction),
				0b010011 => op_mtlo(self, instruction),
				0b011000 => op_mult(self, instruction),
				0b011001 => op_multu(self, instruction),
				0b011010 => op_div(self, instruction),
				0b011011 => op_divu(self, instruction),
				0b100000 => op_add(self, instruction),
				0b100001 => op_addu(self, instruction),
				0b100010 => op_sub(self, instruction),
				0b100011 => op_subu(self, instruction),
				0b100100 => op_and(self, instruction),
				0b100101 => op_or(self, instruction),
				0b100110 => op_xor(self, instruction),
				0b100111 => op_nor(self, instruction),
				0b101010 => op_slt(self, instruction),
				0b101011 => op_sltu(self, instruction),
				_ => op_reserved(self, instruction)
			},
			0b000001 => op_bxx(self, instruction),
			0b000010 => op_j(self, instruction),
			0b000011 => op_jal(self, instruction),
			0b000100 => op_beq(self, instruction),
			0b000101 => op_bne(self, instruction),
			0b000110 => op_blez(self, instruction),
			0b000111 => op_bgtz(self, instruction),
			0b001000 => op_addi(self, instruction),
			0b001001 => op_addiu(self, instruction),
			0b001010 => op_slti(self, instruction),
			0b001011 => op_sltiu(self, instruction),
			0b001100 => op_andi(self, instruction),
			0b001101 => op_ori(self, instruction),
			0b001110 => op_xori(self, instruction),
			0b001111 => op_lui(self, instruction),
			0b010000 => op_cop0(self, instruction),
			0b010001 => op_cop1(self),
			0b010010 => op_cop2(instruction),
			0b010011 => op_cop3(self),
			0b100000 => op_lb(self, instruction),
			0b100001 => op_lh(self, instruction),
			0b100010 => op_lwl(self, instruction),
			0b100011 => op_lw(self, instruction),
			0b100100 => op_lbu(self, instruction),
			0b100101 => op_lhu(self, instruction),
			0b100110 => op_lwr(self, instruction),
			0b101000 => op_sb(self, instruction),
			0b101001 => op_sh(self, instruction),
			0b101010 => op_swl(self, instruction),
			0b101011 => op_sw(self, instruction),
			0b101110 => op_swr(self, instruction),
			0b110000 => op_lwc0(self),
			0b110001 => op_lwc1(self),
			0b110010 => op_lwc2(instruction),
			0b110011 => op_lwc3(self),
			0b111000 => op_swc0(self),
			0b111001 => op_swc1(self),
			0b111010 => op_swc2(instruction),
			0b111011 => op_swc3(self),
			_ => op_reserved(self, instruction)
		}
	}

	pub fn load8(&mut self, address: u32) -> u8 {
		self.interconnect.load8(address)
	}

	pub fn load16(&mut self, address: u32) -> u16 {
		if address % 2 != 0 {
			self.enter_exception(Exception::ADDRLOAD)
		}

		self.interconnect.load16(address)
	}

	pub fn load32(&mut self, address: u32) -> u32 {
		if address % 4 != 0 {
			self.enter_exception(Exception::ADDRLOAD)
		}

		self.interconnect.load32(address)
	}

	pub fn store8(&mut self, address: u32, data: u8) {
		self.interconnect.store8(address, data);
	}

	pub fn store16(&mut self, address: u32, data: u16) {
		if address % 2 != 0 {
			self.enter_exception(Exception::ADDRSTORE)
		}

		self.interconnect.store16(address, data);
	}

	pub fn store32(&mut self, address: u32, data: u32) {
		if address % 4 != 0 {
			self.enter_exception(Exception::ADDRSTORE)
		}

		self.interconnect.store32(address, data);
	}

	pub fn reg(&self, index: usize) -> u32 {
		if index > 0 {
			self.regs[index]
		} else {
			0
		}
	}

	pub fn set_reg(&mut self, index: usize, data: u32) {
		if index > 0 {
			self.load_delay_regs[index] = data;
		}
	}

	pub fn cop0_reg(&mut self, index: usize) -> u32 {
		self.cop0.reg(index)
	}

	pub fn set_cop0_reg(&mut self, index: usize, data: u32) {
		self.cop0.set_reg(index, data);
	}

	pub fn branch(&mut self, offset: u32) {
		self.branch_delay_enabled = true;
		self.branch_delay_pc = self.pc.wrapping_add(4).wrapping_add(offset << 2);
	}

	pub fn enter_exception(&mut self, cause: Exception) {
		println!("EXCEPTION: {:?} at 0x{:08x}", cause, self.pc);
		self.cop0.enter_exception(self.pc, cause as u8, self.branch_delay_slot);
		self.pc = self.cop0.exception_handler().wrapping_sub(4);
	}

	pub fn exit_exception(&mut self) {
		self.cop0.exit_exception();
	}
}