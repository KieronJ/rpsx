pub fn bcd_to_u8(value: u8) -> u8 {
    ((value >> 4) * 10) + (value & 0xf)
}

pub fn u8_to_bcd(value: u8) -> u8 {
    ((value / 10) << 4) | (value % 10)
}