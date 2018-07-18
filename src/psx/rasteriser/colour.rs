use super::Vector3f;

#[derive(Clone, Copy)]
pub struct Colour {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: bool,
}

impl Colour {
    pub fn new(r: f32, g: f32, b: f32, a: bool) -> Colour {
        Colour {
            r: r,
            g: g,
            b: b,
            a: a,
        }
    }

    pub fn from_u8(r: u8, g: u8, b: u8, a: bool) -> Colour {
        let r = (r as f32) / 255.0;
        let g = (g as f32) / 255.0;
        let b = (b as f32) / 255.0;

        Colour::new(r, g, b, a)
    }

    pub fn from_u16(colour: u16) -> Colour {
        let r = ((colour << 3) as u8) & 0xf8;
        let g = ((colour >> 2) as u8) & 0xf8;
        let b = ((colour >> 7) as u8) & 0xf8;
        let a = (colour >> 15) != 0;

        Colour::from_u8(r, g, b, a)
    }

    pub fn from_u32(colour: u32) -> Colour {
        let r = colour as u8;
        let g = (colour >> 8) as u8;
        let b = (colour >> 16) as u8;

        Colour::from_u8(r, g, b, false)
    }

    pub fn to_u8(&self) -> (u8, u8, u8, bool) {
        let r = (self.r * 255.0) as u8;
        let g = (self.g * 255.0) as u8;
        let b = (self.b * 255.0) as u8;

        (r, g, b, self.a)
    }

    pub fn to_u16(&self) -> u16 {
        let mut pixel = 0;

        let (r, g, b, _) = self.to_u8();

        pixel |= ((r as u16) & 0xf8) >> 3;
        pixel |= ((g as u16) & 0xf8) << 2;
        pixel |= ((b as u16) & 0xf8) << 7;

        pixel
    }

    pub fn interpolate_colour(c: &[Colour], v: Vector3f) -> Colour {
        let r = c[0].r * v.x + c[1].r * v.y + c[2].r * v.z;
        let g = c[0].g * v.x + c[1].g * v.y + c[2].g * v.z;
        let b = c[0].b * v.x + c[1].b * v.y + c[2].b * v.z;

        Colour::new(r, g, b, false)
    }
}