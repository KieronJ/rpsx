#[derive(Clone, Copy)]
pub struct Vector2f {
    pub x: f32,
    pub y: f32,
}

impl Vector2f {
    pub fn new(x: f32, y: f32) -> Vector2f {
        Vector2f {
            x: x,
            y: y,
        }
    }

    pub fn sub(&self, v: Vector2f) -> Vector2f {
        Vector2f::new(self.x - v.x, self.y - v.y)
    }

    pub fn dot(a: Vector2f, b: Vector2f) -> f32 {
        a.x * b.x + a.y * b.y
    }
}