use std::io::{self, Read, Seek};
use std::{fs, path};

use super::Container;

pub struct Bin {
    file: fs::File,
}

impl Container for Bin {
    fn open(filepath: &path::Path) -> Result<Box<Self>, String> {
        if !filepath.exists() {
            return Err("File does not exist.".to_string());
        }

        let file;

        match fs::File::open(filepath) {
            Ok(f) => file = f,
            Err(e) => return Err(e.to_string()),
        };

        Ok(Box::new(Self { file: file }))
    }

    fn read(&mut self, lba: usize, buffer: &mut [u8; 2352]) -> Result<(), String> {
        let offset = (lba * 2352) as u64;

        if let Err(e) = self.file.seek(io::SeekFrom::Start(offset)) {
            return Err(e.to_string());
        }

        if let Err(e) = self.file.read_exact(buffer) {
            return Err(e.to_string());
        }

        Ok(())
    }
}