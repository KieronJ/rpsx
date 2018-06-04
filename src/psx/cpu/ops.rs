#[derive(Clone, Copy)]
pub struct Instruction(u32);

impl Instruction {
    pub fn opcode(&self) -> usize {
        ((self.0 >> 26) & 0x3f) as usize
    }

    pub fn rs(&self) -> usize {
        ((self.0 >> 21) & 0x1f) as usize
    }

    pub fn rt(&self) -> usize {
        ((self.0 >> 16) & 0x1f) as usize
    }

    pub fn rd(&self) -> usize {
        ((self.0 >> 11) & 0x1f) as usize
    }

    pub fn shift(&self) -> usize {
        ((self.0 >> 6) & 0x1f) as usize
    }

    pub fn imm(&self) -> u32 {
        (self.0 & 0xffff) as u32
    }

    pub fn imm_se(&self) -> u32 {
        (self.0 & 0xffff) as i16 as u32
    }

    pub fn function(&self) -> usize {
        (self.0 & 0x3f) as usize
    }

    pub fn target(&self) -> u32 {
        (self.0 & 0x3ff_ffff)
    }
}

#[derive(Clone, Copy)]
pub enum Operation {
    Sll(usize, usize, usize),
    Srl(usize, usize, usize),
    Sra(usize, usize, usize),
    Sllv(usize, usize, usize),
    Srlv(usize, usize, usize),
    Srav(usize, usize, usize),
    Jr(usize),
    Jalr(usize, usize),
    Syscall,
    Break,
    Mfhi(usize),
    Mthi(usize),
    Mflo(usize),
    Mtlo(usize),
    Multu(usize, usize),
    Div(usize, usize),
    Divu(usize, usize),
    Add(usize, usize, usize),
    Addu(usize, usize, usize),
    Sub(usize, usize, usize),
    Subu(usize, usize, usize),
    And(usize, usize, usize),
    Or(usize, usize, usize),
    Xor(usize, usize, usize),
    Nor(usize, usize, usize),
    Slt(usize, usize, usize),
    Sltu(usize, usize, usize),
    Bltz(usize, u32),
    Bgez(usize, u32),
    Bltzal(usize, u32),
    Bgezal(usize, u32),
    J(u32),
    Jal(u32),
    Beq(usize, usize, u32),
    Bne(usize, usize, u32),
    Blez(usize, u32),
    Bgtz(usize, u32),
    Addi(usize, usize, u32),
    Addiu(usize, usize, u32),
    Slti(usize, usize, u32),
    Sltiu(usize, usize, u32),
    Andi(usize, usize, u32),
    Ori(usize, usize, u32),
    Xori(usize, usize, u32),
    Lui(usize, u32),
    Mfc0(usize, usize),
    Mtc0(usize, usize),
    Rfe,
    Lb(usize, usize, u32),
    Lh(usize, usize, u32),
    Lwl(usize, usize, u32),
    Lw(usize, usize, u32),
    Lbu(usize, usize, u32),
    Lhu(usize, usize, u32),
    Lwr(usize, usize, u32),
    Sb(usize, usize, u32),
    Sh(usize, usize, u32),
    Swl(usize, usize, u32),
    Sw(usize, usize, u32),
    Swr(usize, usize, u32),
    Unknown(u32),
}

impl From<u32> for Operation {
    fn from(opcode: u32) -> Operation {
        use self::Operation::*;

        let i = Instruction(opcode);

        match i.opcode() {
            0x00 => match i.function() {
                0x00 => Sll(i.rd(), i.rt(), i.shift()),
                0x02 => Srl(i.rd(), i.rt(), i.shift()),
                0x03 => Sra(i.rd(), i.rt(), i.shift()),
                0x04 => Sllv(i.rd(), i.rt(), i.rs()),
                0x06 => Srlv(i.rd(), i.rt(), i.rs()),
                0x07 => Srav(i.rd(), i.rt(), i.rs()),
                0x08 => Jr(i.rs()),
                0x09 => Jalr(i.rd(), i.rs()),
                0x0c => Syscall,
                0x0d => Break,
                0x10 => Mfhi(i.rd()),
                0x11 => Mthi(i.rs()),
                0x12 => Mflo(i.rd()),
                0x13 => Mtlo(i.rs()),
                0x19 => Multu(i.rs(), i.rt()),
                0x1a => Div(i.rs(), i.rt()),
                0x1b => Divu(i.rs(), i.rt()),
                0x20 => Add(i.rd(), i.rs(), i.rt()),
                0x21 => Addu(i.rd(), i.rs(), i.rt()),
                0x22 => Sub(i.rd(), i.rs(), i.rt()),
                0x23 => Subu(i.rd(), i.rs(), i.rt()),
                0x24 => And(i.rd(), i.rs(), i.rt()),
                0x25 => Or(i.rd(), i.rs(), i.rt()),
                0x26 => Xor(i.rd(), i.rs(), i.rt()),
                0x27 => Nor(i.rd(), i.rs(), i.rt()),
                0x2a => Slt(i.rd(), i.rs(), i.rt()),
                0x2b => Sltu(i.rd(), i.rs(), i.rt()),
                _ => Unknown(opcode),
            },
            0x01 => match i.rt() {
                0x00 => Bltz(i.rs(), i.imm_se()),
                0x01 => Bgez(i.rs(), i.imm_se()),
                0x10 => Bltzal(i.rs(), i.imm_se()),
                0x11 => Bgezal(i.rs(), i.imm_se()),
                _ => Unknown(opcode),
            },
            0x02 => J(i.target()),
            0x03 => Jal(i.target()),
            0x04 => Beq(i.rs(), i.rt(), i.imm_se()),
            0x05 => Bne(i.rs(), i.rt(), i.imm_se()),
            0x06 => Blez(i.rs(), i.imm_se()),
            0x07 => Bgtz(i.rs(), i.imm_se()),
            0x08 => Addi(i.rt(), i.rs(), i.imm_se()),
            0x09 => Addiu(i.rt(), i.rs(), i.imm_se()),
            0x0a => Slti(i.rt(), i.rs(), i.imm_se()),
            0x0b => Sltiu(i.rt(), i.rs(), i.imm_se()),
            0x0c => Andi(i.rt(), i.rs(), i.imm()),
            0x0d => Ori(i.rt(), i.rs(), i.imm()),
            0x0e => Xori(i.rt(), i.rs(), i.imm()),
            0x0f => Lui(i.rt(), i.imm()),
            0x10 => match i.rs() {
                0x00 => Mfc0(i.rd(), i.rt()),
                0x04 => Mtc0(i.rd(), i.rt()),
                0x10 => Rfe,
                _ => Unknown(opcode),
            },
            0x20 => Lb(i.rt(), i.rs(), i.imm_se()),
            0x21 => Lh(i.rt(), i.rs(), i.imm_se()),
            0x22 => Lwl(i.rt(), i.rs(), i.imm_se()),
            0x23 => Lw(i.rt(), i.rs(), i.imm_se()),
            0x24 => Lbu(i.rt(), i.rs(), i.imm_se()),
            0x25 => Lhu(i.rt(), i.rs(), i.imm_se()),
            0x26 => Lwr(i.rt(), i.rs(), i.imm_se()),
            0x28 => Sb(i.rt(), i.rs(), i.imm_se()),
            0x29 => Sh(i.rt(), i.rs(), i.imm_se()),
            0x2a => Swl(i.rt(), i.rs(), i.imm_se()),
            0x2b => Sw(i.rt(), i.rs(), i.imm_se()),
            0x2e => Swr(i.rt(), i.rs(), i.imm_se()),
            _ => Unknown(opcode),
        }
    }
}