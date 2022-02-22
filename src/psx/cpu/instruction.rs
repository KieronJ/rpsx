#[derive(Clone, Copy)]
pub struct Instruction(pub u32);

impl Instruction {
    pub fn opcode(self) -> usize {
        ((self.0 >> 26) & 0x3f) as usize
    }

    pub fn rs(self) -> usize {
        ((self.0 >> 21) & 0x1f) as usize
    }

    pub fn rt(self) -> usize {
        ((self.0 >> 16) & 0x1f) as usize
    }

    pub fn rd(self) -> usize {
        ((self.0 >> 11) & 0x1f) as usize
    }

    pub fn shift(self) -> usize {
        ((self.0 >> 6) & 0x1f) as usize
    }

    pub fn imm(self) -> u32 {
        (self.0 & 0xffff) as u32
    }

    pub fn imm_se(self) -> u32 {
        (self.0 & 0xffff) as i16 as u32
    }

    pub fn function(self) -> usize {
        (self.0 & 0x3f) as usize
    }

    pub fn target(self) -> u32 {
        self.0 & 0x3ff_ffff
    }
}