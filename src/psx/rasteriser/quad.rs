use super::Vertex;

#[derive(Clone, Copy)]
pub struct Quad {
    pub vertices: [Vertex; 4],
}

impl Quad {
    pub fn new(v1: Vertex, v2: Vertex, v3: Vertex, v4: Vertex) -> Quad {
        Quad {
            vertices: [v1, v2, v3, v4],
        }
    }
}