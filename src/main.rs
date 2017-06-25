extern crate byteorder;

mod cpu;

use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use cpu::CPU;

fn main() {
	let bios_filepath = env::args().nth(1).unwrap();
	let bios = read_file(bios_filepath);

	let mut cpu = CPU::new(bios);

	loop {
		cpu.run();
	}
}

pub fn read_file<P: AsRef<Path>>(path: P) -> Box<[u8]> {
	let mut file = File::open(path).unwrap();
	let mut file_buffer = Vec::new();
	file.read_to_end(&mut file_buffer).unwrap();
	file_buffer.into_boxed_slice()
}