use byteorder::{LittleEndian, ByteOrder};

pub struct Bios {
    data: Box<[u8]>
}

impl Bios {
	pub fn new(bios: Box<[u8]>) -> Bios {
		Bios {
			data: bios
		}
	}

	pub fn load32(&self, address: u32) -> u32 {
		LittleEndian::read_u32(&self.data[address as usize..])
	}

	pub fn store32(&self, address: u32, data: u32) {
		panic!("Store in read-only BIOS range!")
	}
}