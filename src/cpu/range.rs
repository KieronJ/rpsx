pub struct Range {
	start: u32,
	end: u32
}

impl Range {
	pub fn new(start: u32, end: u32) -> Range {
		Range {
			start: start,
			end: end
		}
	}

	pub fn between(&self, address: u32) -> bool {
		(self.start <= address) & (address < self.end)
	}
}