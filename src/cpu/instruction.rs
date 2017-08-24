pub struct Instruction {
	instruction: u32
}

impl Instruction {
	pub fn new(instruction: u32) -> Instruction {
		Instruction {
			instruction: instruction
		}
	}

	pub fn as_bytes(&self) -> u32 {
		self.instruction
	}

	pub fn opcode(&self) -> u32 {
		(self.instruction >> 26)
	}

	pub fn rs(&self) -> usize {
		((self.instruction >> 21) & 0x1f) as usize
	}

	pub fn rt(&self) -> usize {
		((self.instruction >> 16) & 0x1f) as usize
	}

	pub fn rd(&self) -> usize {
		((self.instruction >> 11) & 0x1f) as usize
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