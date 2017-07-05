pub struct Range (pub u32, pub u32);

impl Range {
	pub fn offset(&self, address: u32) -> usize {
		(address - self.0) as usize
	}

	pub fn between(&self, address: u32) -> bool {
		(self.0 <= address) & (address < self.1)
	}
}