use super::Colour;
use super::Vector2f;

#[derive(Clone, Copy)]
pub struct Vertex {
    pub position: Vector2f,
    pub texcoord: Vector2f,
    pub colour: Colour,
}

impl Vertex {
    pub fn new(position: Vector2f, texcoord: Vector2f, colour: Colour) -> Vertex {
        Vertex {
            position: position,
            texcoord: texcoord,
            colour: colour,
        }
    }
}