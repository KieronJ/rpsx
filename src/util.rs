use std::fs::File;
use std::io::Read;
use std::path::Path;

pub fn bcd_to_u8(value: u8) -> u8 {
	((value >> 4) * 10) + (value & 0xf)
}

pub fn u8_to_bcd(value: u8) -> u8 {
	((value / 10) << 4) | (value % 10)
}

pub fn read_file_to_box(filepath: &str) -> Box<[u8]> {
	let path = Path::new(filepath);

	if !path.is_file() {
		panic!("ERROR: file does not exist: {}", path.display())
	}

	let mut file = File::open(path).unwrap();
	let mut file_buffer = Vec::new();
	
	file.read_to_end(&mut file_buffer).unwrap();
	
	file_buffer.into_boxed_slice()
}