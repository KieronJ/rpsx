use super::Timecode;

#[allow(dead_code)]
pub struct Header {
    timecode: Timecode,
    mode: u8,
}

#[allow(dead_code)]
pub struct Subheader {
    file: u8,
    channel: u8,
    submode: u8,
    coding_info: u8,
}