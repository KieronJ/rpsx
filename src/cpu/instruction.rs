pub struct Instruction {
	instruction: u32
}

impl Instruction {
	pub fn new(data: u32) -> Instruction {
		Instruction {
			instruction: data
		}
	}

	pub fn as_bytes(&self) -> u32 {
		self.instruction
	}

	pub fn opcode(&self) -> u32 {
		(self.instruction >> 26) & 0x3f
	}

	pub fn rs(&self) -> u32 {
		(self.instruction >> 21) & 0x1f
	}

	pub fn rt(&self) -> u32 {
		(self.instruction >> 16) & 0x1f
	}

	pub fn rd(&self) -> u32 {
		(self.instruction >> 11) & 0x1f
	}

	pub fn shift(&self) -> u32 {
		(self.instruction >>  6) & 0x1f
	}

	pub fn function(&self) -> u32 {
		self.instruction & 0x3f
	}

	pub fn target(&self) -> u32 {
		self.instruction & 0x03ff_ffff
	}

	pub fn imm(&self) -> u32 {
		self.instruction & 0xffff
	}

	pub fn imm_se(&self) -> u32 {
		((self.instruction & 0xffff) as i16) as u32
	}
}