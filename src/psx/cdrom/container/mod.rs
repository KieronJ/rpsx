mod bin;
mod no_disk;

use std::path;

pub use bin::Bin;
pub use no_disk::NoDisk;

pub trait Container {
    fn open(filepath: &path::Path) -> Result<Box<Self>, String>;
    fn read(&mut self, lba: usize, buffer: &mut [u8; 2352]) -> Result<(), String>;
}