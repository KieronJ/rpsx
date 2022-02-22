use std::cmp;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use byteorder::{ByteOrder, LittleEndian}; 

pub fn bcd_to_u8(value: u8) -> u8 {
    ((value >> 4) * 10) + (value & 0xf)
}

pub fn u8_to_bcd(value: u8) -> u8 {
    ((value / 10) << 4) | (value % 10)
}

pub fn i16_to_f32(value: i16) -> f32 {
    if value >= 0 {
        f32::from(value) / f32::from(i16::max_value())
    } else {
        -f32::from(value) / f32::from(i16::min_value())
    }
}

pub fn f32_to_i16(value: f32) -> i16 {
    if value >= 0.0 {
        (value * f32::from(i16::max_value())) as i16
    } else {
        (-value * f32::from(i16::min_value())) as i16
    }
}

pub fn sign_extend_u16(value: u16, size: usize) -> u16 {
    let sign = 1 << (size - 1);
    let mask = !((1 << size) - 1);

    if (value & sign) != 0 {
        return value | mask;
    }

    return value;
}

pub fn sign_extend_i32(value: i32, size: usize) -> i32 {
    let sign = 1 << (size - 1);
    let mask = !((1 << size) - 1);

    if (value & sign) != 0 {
        return value | mask;
    }

    return value;
}

pub fn clip<T: PartialOrd>(value: T, min: T, max: T) -> T {
    if value <= min {
        return min;
    }

    if value >= max {
        return max;
    }

    return value;
}

pub fn min3<T: Ord>(a: T, b: T, c: T) -> T {
    cmp::min(a, cmp::min(b, c))
}

pub fn max3<T: Ord>(a: T, b: T, c: T) -> T {
    cmp::max(a, cmp::max(b, c))
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

pub fn discard(file: &mut File, size: usize) -> io::Result<()> {
    let mut buffer = vec![0; size];
    file.read_exact(&mut buffer)?;

    Ok(())
}

pub fn read_to_buffer(file: &mut File, size: usize) -> io::Result<Vec<u8>> {
    let mut buffer = vec![0; size];
    file.read_exact(&mut buffer)?;

    Ok(buffer)
}

pub fn read_u8(file: &mut File) -> io::Result<u8> {
    Ok(read_to_buffer(file, 1)?[0])
}

pub fn read_u32(file: &mut File) -> io::Result<u32> {
    Ok(LittleEndian::read_u32(&read_to_buffer(file, 4)?))
}