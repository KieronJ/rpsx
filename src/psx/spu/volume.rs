use crate::util::i16_to_f32;

#[derive(Clone, Copy, Default)]
pub struct Volume {
    pub left: i16,
    pub right: i16,
}

impl Volume {
    pub fn l(self) -> f32 {
        i16_to_f32(self.left)
    }

    pub fn r(self) -> f32 {
        i16_to_f32(self.right)
    }
}
