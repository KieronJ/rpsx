#[derive(Clone, Copy)]
pub struct Vector3i {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Vector3i {
    pub fn new(x: i32, y: i32, z: i32) -> Vector3i {
        Vector3i { x, y, z }
    }
}
