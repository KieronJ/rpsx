use std::path;

use super::Container;

pub struct NoDisk;

impl Container for NoDisk {
    fn open(_: &path::Path) -> Result<Box<Self>, String> {
        Ok(Box::new(Self))
    }

    fn read(&mut self, _: usize, buffer: &mut [u8; 2352]) -> Result<(), String> {
        for i in 0..buffer.len() { buffer[i] = 0; }

        Err("No disk inserted".to_string())
    }
}