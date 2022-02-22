#[derive(Clone, Copy)]
pub struct Colour {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: bool,
}

impl Colour {
    pub fn new(r: u8, g: u8, b: u8, a: bool) -> Colour {
        Colour { r, g, b, a }
    }

    pub fn from_u16(colour: u16) -> Colour {
        let rb = (colour & 0x1f) as u8;
        let gb = ((colour >> 5) & 0x1f) as u8;
        let bb = ((colour >> 10) & 0x1f) as u8;

        let r = (rb << 3) | (rb >> 2);
        let g = (gb << 3) | (gb >> 2);
        let b = (bb << 3) | (bb >> 2);
        let a = (colour >> 15) != 0;

        Colour::new(r, g, b, a)
    }

    pub fn from_u32(colour: u32) -> Colour {
        let r = colour as u8;
        let g = (colour >> 8) as u8;
        let b = (colour >> 16) as u8;

        Colour::new(r, g, b, false)
    }

    pub fn to_u16(self) -> u16 {
        let mut pixel = 0;

        pixel |= ((self.r as u16) & 0xf8) >> 3;
        pixel |= ((self.g as u16) & 0xf8) << 2;
        pixel |= ((self.b as u16) & 0xf8) << 7;
        pixel |= (self.a as u16) << 15;

        pixel
    }

    pub fn to_u32(self) -> u32 {
        let mut pixel = 0;

        pixel |= self.r as u32;
        pixel |= (self.g as u32) << 8;
        pixel |= (self.b as u32) << 16;

        pixel
    }

    pub fn to_f32(self) -> [f32; 3] {
        let r = (self.r as f32) / 255.0;
        let g = (self.g as f32) / 255.0;
        let b = (self.b as f32) / 255.0;

        [r, g, b]
    }

    pub fn r(self) -> i32 {
        self.r as i32
    }

    pub fn g(self) -> i32 {
        self.g as i32
    }

    pub fn b(self) -> i32 {
        self.b as i32
    }
}
