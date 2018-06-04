use std::fs::File;
use std::io::Read;
use std::path::Path;

pub fn read_file_to_box(filepath: &str) -> Box<[u8]>
{
	let path = Path::new(filepath);

	if !path.is_file() {
		panic!("ERROR: file does not exist: {}", path.display())
	}

	let mut file = File::open(path).unwrap();
	let mut file_buffer = Vec::new();
	
	file.read_to_end(&mut file_buffer).unwrap();
	
	file_buffer.into_boxed_slice()
}