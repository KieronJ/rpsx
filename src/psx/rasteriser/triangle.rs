use super::{Colour, Vector2f, Vector3f, Vertex};
use super::util;

#[derive(Clone, Copy)]
pub struct Triangle {
    pub vertices: [Vertex; 3],
    ab: Vector2f,
    ac: Vector2f,
    d00: f32,
    d01: f32,
    d11: f32,
    inv_divisor: f32,
}

impl Triangle {
    pub fn new(v1: Vertex, v2: Vertex, v3: Vertex) -> Triangle {
        let a = v1.position;
        let b = v2.position;
        let c = v3.position;

        let ab = b.sub(a);
        let ac = c.sub(a);

        let d00 = Vector2f::dot(ab, ab) as f32;
        let d01 = Vector2f::dot(ab, ac) as f32;
        let d11 = Vector2f::dot(ac, ac) as f32;

        Triangle {
            vertices: [v1, v2, v3],
            ab: ab,
            ac: ac,
            d00: d00,
            d01: d01,
            d11: d11,
            inv_divisor: 1.0 / (d00 * d11 - d01 * d01),
        }
    }

    pub fn bounding_box(&self) -> (Vector2f, Vector2f) {
        let a = self.vertices[0].position;
        let b = self.vertices[1].position;
        let c = self.vertices[2].position;

        let (minx, maxx) = util::f32_cmp_3(a.x, b.x, c.x);
        let (miny, maxy) = util::f32_cmp_3(a.y, b.y, c.y);

        (Vector2f::new(minx, miny), Vector2f::new(maxx, maxy))
    }

    pub fn barycentric_vector(&self, x: f32, y: f32) -> Vector3f {
        let a = self.vertices[0].position;
        let p = Vector2f::new(x, y);

        let ap = p.sub(a);

        let d20 = Vector2f::dot(ap, self.ab);
        let d21 = Vector2f::dot(ap, self.ac);

        let l1 = (self.d11 * d20 - self.d01 * d21) * self.inv_divisor;
        let l2 = (self.d00 * d21 - self.d01 * d20) * self.inv_divisor;
        let l3 = 1.0 - l1 - l2;

        Vector3f::new(l3, l1, l2)
    }

    pub fn interpolate_colour(&self, v: Vector3f) -> Colour {
        let a = self.vertices[0].colour;
        let b = self.vertices[1].colour;
        let c = self.vertices[2].colour;

        let r = a.r * v.x + b.r * v.y + c.r * v.z;
        let g = a.g * v.x + b.g * v.y + c.g * v.z;
        let b = a.b * v.x + b.b * v.y + c.b * v.z;

        Colour::new(r, g, b)
    }

    pub fn interpolate_texcoord(&self, v: Vector3f) -> Vector2f {
        let a = self.vertices[0].texcoord;
        let b = self.vertices[1].texcoord;
        let c = self.vertices[2].texcoord;

        let x = a.x * v.x + b.x * v.y + c.x * v.z;
        let y = a.y * v.x + b.y * v.y + c.y * v.z;

        Vector2f::new(x, y)
    }
}