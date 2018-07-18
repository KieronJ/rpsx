use super::Vector3f;

#[derive(Clone, Copy)]
pub struct Vector2i {
    pub x: i32,
    pub y: i32,
}

impl Vector2i {
    pub fn new(x: i32, y: i32) -> Vector2i {
        Vector2i {
            x: x,
            y: y,
        }
    }

    pub fn orient2d(a: Vector2i, c: Vector2i, b: Vector2i) -> i32 {
        (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
    }

    pub fn interpolate_texcoord(t: &[Vector2i], v: Vector3f) -> Vector2i {
        let t0x = t[0].x as f32; let t0y = t[0].y as f32;
        let t1x = t[1].x as f32; let t1y = t[1].y as f32;
        let t2x = t[2].x as f32; let t2y = t[2].y as f32;

        let x = t0x * v.x + t1x * v.y + t2x * v.z;
        let y = t0y * v.x + t1y * v.y + t2y * v.z;

        Vector2i::new(x.round() as i32, y.round() as i32)
    }
}