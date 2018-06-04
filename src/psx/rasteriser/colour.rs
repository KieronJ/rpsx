#[derive(Clone, Copy)]
pub struct Colour {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Colour {
    pub fn new(r: f32, g: f32, b: f32) -> Colour {
        Colour {
            r: r,
            g: g,
            b: b,
        }
    }

    pub fn from_u8(r: u8, g: u8, b: u8) -> Colour {
        let rn = (r as f32) / 255.0;
        let gn = (g as f32) / 255.0;
        let bn = (b as f32) / 255.0;

        Colour::new(rn, gn, bn)
    }

    pub fn from_u16(colour: u16) -> Colour {
        let r = ((colour >> 7) as u8) & 0xf8;
        let g = ((colour >> 2) as u8) & 0xf8;
        let b = ((colour << 3) as u8) & 0xf8;

        Colour::from_u8(r, g, b)
    }

    pub fn from_u16_bgr(colour: u16) -> Colour {
        let b = ((colour >> 7) as u8) & 0xf8;
        let g = ((colour >> 2) as u8) & 0xf8;
        let r = ((colour << 3) as u8) & 0xf8;

        Colour::from_u8(r, g, b)
    }

    pub fn from_u32(colour: u32) -> Colour {
        let r = ((colour & 0xff) as f32) / 255.0;
        let g = (((colour >> 8) & 0xff) as f32) / 255.0;
        let b = (((colour >> 16) & 0xff) as f32) / 255.0;

        Colour::new(r, g, b)
    }

    pub fn to_u8(&self) -> (u8, u8, u8) {
        let r = (self.r * 255.0) as u8;
        let g = (self.g * 255.0) as u8;
        let b = (self.b * 255.0) as u8;

        (r, g, b)
    }
}