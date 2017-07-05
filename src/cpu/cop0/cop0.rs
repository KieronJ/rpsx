use super::{Status, Cause};

#[derive(Default)]
pub struct Cop0 {
	bpc: u32,
	bda: u32,
	jumpdest: u32,
	dcic: u32,
	bdam: u32,
	bpcm: u32,
	status: Status,
	cause: Cause
}

impl Cop0 {
	pub fn reg(&self, index: usize) -> u32 {
		match index {
			3 => self.bpc,
			5 => self.bda,
			6 => self.jumpdest,
			7 => self.dcic,
			9 => self.bdam,
			11 => self.bpcm,
			12 => self.status.get_value(),
			13 => self.cause.get_value(),
			_ => panic!("load from unknown cop0 register cop0_r{}", index)
		}
	}

	pub fn set_reg(&mut self, index: usize, data: u32) {
		match index {
			3 => self.bpc = data,
			5 => self.bda = data,
			6 => println!("store to read-only cop0 register"),
			7 => self.dcic = data,
			9 => self.bdam = data,
			11 => self.bpcm = data,
			12 => self.status.set_value(data),
			13 => self.cause.set_value(data),
			_ => panic!("store to unknown cop0 register cop0_r{}", index)
		}
	}

	pub fn isolate_cache(&self) -> bool {
		self.status.isolate_cache()
	}
}