use super::{Status, Cause};

static CPU_PROCESSOR_ID: u32 = 2;

#[derive(Default)]
pub struct Cop0 {
	bpc: u32,
	bda: u32,
	jumpdest: u32,
	dcic: u32,
	badvaddr: u32,
	bdam: u32,
	bpcm: u32,
	status: Status,
	cause: Cause,
	epc: u32
}

impl Cop0 {
	pub fn reset(&mut self, epc: u32) {
		self.epc = epc;
		self.status.reset();
	}

	pub fn reg(&self, index: usize) -> u32 {
		match index {
			3  => self.bpc,
			5  => self.bda,
			6  => self.jumpdest,
			7  => self.dcic,
			8  => self.badvaddr,
			9  => self.bdam,
			11 => self.bpcm,
			12 => self.status.get_value(),
			13 => self.cause.get_value(),
			14 => self.epc,
			15 => CPU_PROCESSOR_ID,
			_  => panic!("load from unknown cop0 register cop0_r{}", index)
		}
	}

	pub fn set_reg(&mut self, index: usize, data: u32) {
		match index {
			3  => self.bpc = data,
			5  => self.bda = data,
			6  => println!("store to read-only cop0 register cop0_r6"),
			7  => self.dcic = data,
			8  => println!("store to read-only cop0 register cop0_r8"),
			9  => self.bdam = data,
			11 => self.bpcm = data,
			12 => self.status.set_value(data),
			13 => self.cause.set_value(data),
			14 => println!("store to read-only cop0 register cop0_r14"),
			15 => println!("store to read-only cop0 register cop0_r15"),
			_  => panic!("store to unknown cop0 register cop0_r{}", index)
		}
	}

	pub fn isolate_cache(&self) -> bool {
		self.status.isolate_cache()
	}
}