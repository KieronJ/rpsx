#[derive(Default)]
pub struct Cause {
	branch_delay: bool,
	coprocessor_exception: u8,
	interrupt_pending: u8,
	exception_code: u8
}

impl Cause {
	pub fn get_value(&self) -> u32 {
		let mut value  = (self.branch_delay                   as u32) << 31;
				value |= ((self.coprocessor_exception & 0b11) as u32) << 28;
				value |= (self.interrupt_pending              as u32) <<  8;
				value |  ((self.exception_code & 0b11111)     as u32) <<  2
	}

	pub fn set_value(&mut self, data: u32) {
		self.interrupt_pending &= 0xfc;
		self.interrupt_pending |= ((data >> 8) & 0x03) as u8;
	}
}