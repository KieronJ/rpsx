pub struct Range (pub u32, pub u32);

impl Range {
	pub fn offset(&self, address: u32) -> u32 {
		address - self.0
	}

	pub fn between(&self, address: u32) -> bool {
		(self.0 <= address) & (address < self.1)
	}
}