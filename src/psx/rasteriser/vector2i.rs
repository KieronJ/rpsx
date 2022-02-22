#[derive(Clone, Copy)]
pub struct Vector2i {
    pub x: i32,
    pub y: i32,
}

impl Vector2i {
    pub fn new(x: i32, y: i32) -> Vector2i {
        Vector2i { x, y }
    }

    pub fn orient2d(a: Vector2i, b: Vector2i, c: Vector2i) -> i32 {
        (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
    }
}
