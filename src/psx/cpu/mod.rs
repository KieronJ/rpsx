mod cop0;
pub mod ops;

use super::bus::{Bus, BusWidth};
use super::interrupt::{Interrupt, InterruptRegister};

use self::ops::Operation;
use self::cop0::{Cop0, Exception};

pub const REGISTERS: [&str; 32] = ["$zero",
                                   "$at",
                                   "$v0", "$v1",
                                   "$a0", "$a1", "$a2", "$a3",
                                   "$t0", "$t1", "$t2", "$t3", "$t4", "$t5", "$t6", "$t7",
                                   "$s0", "$s1", "$s2", "$s3", "$s4", "$s5", "$s6", "$s7",
                                   "$t8", "$t9",
                                   "$k0", "$k1",
                                   "$gp",
                                   "$sp",
                                   "$fp",
                                   "$ra"];

enum VirtualRegion {
    Kuseg,
    Kseg0,
    Kseg1,
    Kseg2,
}

impl VirtualRegion {
    pub fn from_u32(address: u32) -> VirtualRegion
    {
        match address >> 29 {
            0b000...0b011 => VirtualRegion::Kuseg,
            0b100         => VirtualRegion::Kseg0,
            0b101         => VirtualRegion::Kseg1,
            0b110...0b111 => VirtualRegion::Kseg2,
            _ => unreachable!()
        }
    }

    pub fn translate_address(&self, address: u32) -> u32
    {
        match self {
            VirtualRegion::Kuseg => address,
            VirtualRegion::Kseg0 => address & 0x7fff_ffff,
            VirtualRegion::Kseg1 => address & 0x1fff_ffff,
            VirtualRegion::Kseg2 => address,
        }
    }
}

pub struct R3000A {
    pc: u32,
    new_pc: u32,
    current_pc: u32,

    last_load: u32,
    last_store: u32,

    branch: bool,
    branch_delay: bool,

    regs: [u32; 32],
    ld_slot: (usize, u32),

    hi: u32,
    lo: u32,

    cop0: Cop0,

    bus: Bus,

    istat: InterruptRegister,
    imask: InterruptRegister,
}

impl R3000A {
    pub fn new(bus: Bus) -> R3000A
    {
        R3000A {
            pc: 0,
            new_pc: 0,
            current_pc: 0,

            last_load: 0,
            last_store: 0,

            branch: false,
            branch_delay: false,

            regs: [0; 32],
            ld_slot: (0, 0),

            hi: 0,
            lo: 0,

            cop0: Cop0::new(),

            bus: bus,

            istat: InterruptRegister::new(),
            imask: InterruptRegister::new(),
        }
    }

    pub fn bus(&mut self) -> &mut Bus {
        &mut self.bus
    }

    pub fn reset(&mut self)
    {
        self.cop0.reset(self.current_pc);

        self.pc = 0xbfc0_0000;
        self.new_pc = self.pc.wrapping_add(4);
        self.current_pc = self.pc;
    }

    pub fn run(&mut self)
    {
        if self.pc & 0x03 != 0 {
            self.enter_exception(Exception::AddrLoad);
            return;
        }

        self.last_load = 0;
        self.last_store = 0;

        let op: Operation = self.fetch32().into();

        self.current_pc = self.pc;
        self.pc = self.new_pc;
        self.new_pc = self.pc.wrapping_add(4);

        self.branch_delay = self.branch;
        self.branch = false;

        let iec = self.cop0.iec();
        let im = self.cop0.im();

        if iec && im {
            self.enter_exception(Exception::Interrupt);
            return;
        }

        self.execute(op);
    }

    pub fn check_interrupts(&self) -> bool {
        (self.imask.read() & self.istat.read()) != 0
    }

    pub fn set_interrupt(&mut self, interrupt: Interrupt) {
        self.istat.set_interrupt(interrupt);

        self.update_irq();
    }

    fn update_irq(&mut self) {
        if self.check_interrupts() {
            self.cop0.set_interrupt_bit();
        } else {
            self.cop0.clear_interrupt_bit();
        }
    }

    fn execute(&mut self, op: Operation) {
        use self::Operation::*;

        match op {
            Sll(rd, rt, shift) => self.op_sll(rd, rt, shift),
            Srl(rd, rt, shift) => self.op_srl(rd, rt, shift),
            Sra(rd, rt, shift) => self.op_sra(rd, rt, shift),
            Sllv(rd, rt, rs) => self.op_sllv(rd, rt, rs),
            Srlv(rd, rt, rs) => self.op_srlv(rd, rt, rs),
            Srav(rd, rt, rs) => self.op_srav(rd, rt, rs),
            Jr(rs) => self.op_jr(rs),
            Jalr(rd, rs) => self.op_jalr(rd, rs),
            Syscall => self.op_syscall(),
            Break => self.op_break(),
            Mfhi(rd) => self.op_mfhi(rd),
            Mthi(rs) => self.op_mthi(rs),
            Mflo(rd) => self.op_mflo(rd),
            Mtlo(rs) => self.op_mtlo(rs),
            Multu(rs, rt) => self.op_multu(rs, rt),
            Div(rs, rt) => self.op_div(rs, rt),
            Divu(rs, rt) => self.op_divu(rs, rt),
            Add(rd, rs, rt) => self.op_add(rd, rs, rt),
            Addu(rd, rs, rt) => self.op_addu(rd, rs, rt),
            Sub(rd, rs, rt) => self.op_sub(rd, rs, rt),
            Subu(rd, rs, rt) => self.op_subu(rd, rs, rt),
            And(rd, rs, rt) => self.op_and(rd, rs, rt),
            Or(rd, rs, rt) => self.op_or(rd, rs, rt),
            Xor(rd, rs, rt) => self.op_xor(rd, rs, rt),
            Nor(rd, rs, rt) => self.op_nor(rd, rs, rt),
            Slt(rd, rs, rt) => self.op_slt(rd, rs, rt),
            Sltu(rd, rs, rt) => self.op_sltu(rd, rs, rt),
            Bltz(rs, offset) => self.op_bltz(rs, offset),
            Bgez(rs, offset) => self.op_bgez(rs, offset),
            Bltzal(rs, offset) => self.op_bltzal(rs, offset),
            Bgezal(rs, offset) => self.op_bgezal(rs, offset),
            J(target) => self.op_j(target),
            Jal(target) => self.op_jal(target),
            Beq(rs, rt, offset) => self.op_beq(rs, rt, offset),
            Bne(rs, rt, offset) => self.op_bne(rs, rt, offset),
            Blez(rs, offset) => self.op_blez(rs, offset),
            Bgtz(rs, offset) => self.op_bgtz(rs, offset),
            Addi(rt, rs, imm) => self.op_addi(rt, rs, imm),
            Addiu(rt, rs, imm) => self.op_addiu(rt, rs, imm),
            Slti(rt, rs, imm) => self.op_slti(rt, rs, imm),
            Sltiu(rt, rs, imm) => self.op_sltiu(rt, rs, imm),
            Andi(rt, rs, imm) => self.op_andi(rt, rs, imm),
            Ori(rt, rs, imm) => self.op_ori(rt, rs, imm),
            Xori(rt, rs, imm) => self.op_xori(rt, rs, imm),
            Lui(rt, imm) => self.op_lui(rt, imm),
            Mfc0(rd, rt) => self.op_mfc0(rd, rt),
            Mtc0(rd, rt) => self.op_mtc0(rd, rt),
            Rfe => self.op_rfe(),
            Mfc2 => println!("Mfc2"),
            Cfc2 => println!("Cfc2"),
            Mtc2 => println!("Mtc2"),
            Ctc2 => println!("Ctc2"),
            Bc2f => println!("Bc2f"),
            Bc2t => println!("Bc2t"),
            Cop2 => println!("Cop2"),
            Lb(rt, rs, offset) => self.op_lb(rt, rs, offset),
            Lh(rt, rs, offset) => self.op_lh(rt, rs, offset),
            Lwl(rt, rs, offset) => self.op_lwl(rt, rs, offset),
            Lw(rt, rs, offset) => self.op_lw(rt, rs, offset),
            Lbu(rt, rs, offset) => self.op_lbu(rt, rs, offset),
            Lhu(rt, rs, offset) => self.op_lhu(rt, rs, offset),
            Lwr(rt, rs, offset) => self.op_lwr(rt, rs, offset),
            Sb(rt, rs, offset) => self.op_sb(rt, rs, offset),
            Sh(rt, rs, offset) => self.op_sh(rt, rs, offset),
            Swl(rt, rs, offset) => self.op_swl(rt, rs, offset),
            Sw(rt, rs, offset) => self.op_sw(rt, rs, offset),
            Swr(rt, rs, offset) => self.op_swr(rt, rs, offset),
            Unknown(instruction) => panic!("[CPU] [ERROR] Unknown instruction 0x{:08x} (0x{:02x}:{:02x})", instruction, instruction >> 26, instruction & 0x3f),
        }
    }

    fn op_sll(&mut self, rd: usize, rt: usize, shift: usize)
    {
        let v = self.reg(rt) << shift;

        self.execute_load_delay();

        self.set_reg(rd, v);
    }

    fn op_srl(&mut self, rd: usize, rt: usize, shift: usize)
    {
        let v = self.reg(rt) >> shift;

        self.execute_load_delay();

        self.set_reg(rd, v);
    }

    fn op_sra(&mut self, rd: usize, rt: usize, shift: usize)
    {
        let v = (self.reg(rt) as i32 >> shift) as u32;

        self.execute_load_delay();

        self.set_reg(rd, v);
    }

    fn op_sllv(&mut self, rd: usize, rt: usize, rs: usize)
    {
        let v = self.reg(rt) << self.reg(rs);

        self.execute_load_delay();

        self.set_reg(rd, v);
    }

    fn op_srlv(&mut self, rd: usize, rt: usize, rs: usize)
    {
        let v = self.reg(rt) >> self.reg(rs);

        self.execute_load_delay();

        self.set_reg(rd, v);
    }

    fn op_srav(&mut self, rd: usize, rt: usize, rs: usize)
    {
        let v = (self.reg(rt) as i32 >> self.reg(rs)) as u32;

        self.execute_load_delay();

        self.set_reg(rd, v);
    }

    fn op_jr(&mut self, rs: usize)
    {
        self.branch = true;
        self.new_pc = self.reg(rs);
        
        self.execute_load_delay();
    }

    fn op_jalr(&mut self, rd: usize, rs: usize)
    {
        let pc = self.pc;

        self.branch = true;
        self.new_pc = self.reg(rs);

        self.execute_load_delay();

        self.set_reg(rd, pc.wrapping_add(4));
    }

    fn op_syscall(&mut self) {
        self.enter_exception(Exception::Syscall);

        self.execute_load_delay();
    }

    fn op_break(&mut self) {
        self.enter_exception(Exception::Breakpoint);

        self.execute_load_delay();
    }

    fn op_mfhi(&mut self, rd: usize) {
        let hi = self.hi;

        self.execute_load_delay();

        self.set_reg(rd, hi);
    }

    fn op_mthi(&mut self, rs: usize) {
        self.hi = self.reg(rs);

        self.execute_load_delay();
    }

    fn op_mflo(&mut self, rd: usize) {
        let lo = self.lo;

        self.execute_load_delay();

        self.set_reg(rd, lo);
    }

    fn op_mtlo(&mut self, rs: usize) {
        self.lo = self.reg(rs);

        self.execute_load_delay();
    }

    fn op_multu(&mut self, rs: usize, rt: usize) {
        let r = self.reg(rs) as u64 * self.reg(rt) as u64;

        self.hi = (r >> 32) as u32;
        self.lo = r as u32;

        self.execute_load_delay();
    }

    fn op_div(&mut self, rs: usize, rt: usize) {
        let n = self.reg(rs) as i32;
        let d = self.reg(rt) as i32;

        if d == 0 {
            self.hi = n as u32;

            if n >= 0 {
                self.lo = 0xffffffff;
            } else {
                self.lo = 0x00000001;
            }
        } else if n as u32 == 0x80000000 && d as u32 == 0xffffffff {
            self.hi = 0;
            self.lo = 0x80000000;
        } else {
            self.hi = (n % d) as u32;
            self.lo = (n / d) as u32;
        }

        self.execute_load_delay();
    }

    fn op_divu(&mut self, rs: usize, rt: usize) {
        let n = self.reg(rs);
        let d = self.reg(rt);

        if d == 0 {
            self.hi = n;
            self.lo = 0xffffffff;
        } else {
            self.hi = n % d;
            self.lo = n / d;
        }

        self.execute_load_delay();
    }

    fn op_add(&mut self, rd: usize, rs: usize, rt: usize)
    {
        let v = (self.reg(rs) as i32).overflowing_add(self.reg(rt) as i32);

        self.execute_load_delay();

        if v.1 {
            self.enter_exception(Exception::Overflow);
        } else {
            self.set_reg(rd, v.0 as u32);
        }
    }

    fn op_addu(&mut self, rd: usize, rs: usize, rt: usize)
    {
        let v = self.reg(rs).wrapping_add(self.reg(rt));

        self.execute_load_delay();

        self.set_reg(rd, v);
    }

    fn op_sub(&mut self, rd: usize, rs: usize, rt: usize)
    {
        let v = (self.reg(rs) as i32).overflowing_sub(self.reg(rt) as i32);

        self.execute_load_delay();

        if v.1 {
            self.enter_exception(Exception::Overflow);
        } else {
            self.set_reg(rd, v.0 as u32);
        }
    }

    fn op_subu(&mut self, rd: usize, rs: usize, rt: usize)
    {
        let v = self.reg(rs).wrapping_sub(self.reg(rt));

        self.execute_load_delay();

        self.set_reg(rd, v);
    }

    fn op_and(&mut self, rd: usize, rs: usize, rt: usize)
    {
        let v = self.reg(rs) & self.reg(rt);

        self.execute_load_delay();

        self.set_reg(rd, v);
    }

    fn op_or(&mut self, rd: usize, rs: usize, rt: usize)
    {
        let v = self.reg(rs) | self.reg(rt);

        self.execute_load_delay();

        self.set_reg(rd, v);
    }

    fn op_xor(&mut self, rd: usize, rs: usize, rt: usize)
    {
        let v = self.reg(rs) ^ self.reg(rt);

        self.execute_load_delay();

        self.set_reg(rd, v);
    }

    fn op_nor(&mut self, rd: usize, rs: usize, rt: usize)
    {
        let v = !(self.reg(rs) | self.reg(rt));

        self.execute_load_delay();

        self.set_reg(rd, v);
    }

    fn op_slt(&mut self, rd: usize, rs: usize, rt: usize)
    {
        let v = ((self.reg(rs) as i32) < (self.reg(rt) as i32)) as u32;

        self.execute_load_delay();

        self.set_reg(rd, v);
    }

    fn op_sltu(&mut self, rd: usize, rs: usize, rt: usize)
    {
        let v = (self.reg(rs) < self.reg(rt)) as u32;

        self.execute_load_delay();

        self.set_reg(rd, v);
    }

    fn op_bltz(&mut self, rs: usize, offset: u32) {
        if (self.reg(rs) as i32) < 0 {
            self.branch(offset);
        }

        self.execute_load_delay();
    }

    fn op_bgez(&mut self, rs: usize, offset: u32) {
        if self.reg(rs) as i32 >= 0 {
            self.branch(offset);
        }

        self.execute_load_delay();
    }

    fn op_bltzal(&mut self, rs: usize, offset: u32) {
        let s = self.reg(rs) as i32;

        self.execute_load_delay();

        if s < 0 {
            let pc = self.pc;
            self.set_reg(31, pc.wrapping_add(4));

            self.branch(offset);
        }
    }

    fn op_bgezal(&mut self, rs: usize, offset: u32) {
        let s = self.reg(rs) as i32;

        self.execute_load_delay();

        if s >= 0 {
            let pc = self.pc;
            self.set_reg(31, pc.wrapping_add(4));

            self.branch(offset);
        }
    }

    fn op_j(&mut self, target: u32)
    {
        self.branch = true;
        self.new_pc = (self.pc & 0xf000_0000) | (target << 2);

        self.execute_load_delay();
    }

    fn op_jal(&mut self, target: u32)
    {
        let pc = self.pc;

        self.branch = true;
        self.new_pc = (self.pc & 0xf000_0000) | (target << 2);

        self.execute_load_delay();

        self.set_reg(31, pc.wrapping_add(4));
    }

    fn op_beq(&mut self, rs: usize, rt: usize, offset: u32)
    {
        if self.reg(rs) == self.reg(rt) {
            self.branch(offset);
        }

        self.execute_load_delay();
    }

    fn op_bne(&mut self, rs: usize, rt: usize, offset: u32)
    {
        if self.reg(rs) != self.reg(rt) {
            self.branch(offset);
        }

        self.execute_load_delay();
    }

    fn op_blez(&mut self, rs: usize, offset: u32)
    {
        if self.reg(rs) as i32 <= 0 {
            self.branch(offset);
        }

        self.execute_load_delay();
    }

    fn op_bgtz(&mut self, rs: usize, offset: u32)
    {
        if self.reg(rs) as i32 > 0 {
            self.branch(offset);
        }

        self.execute_load_delay();
    }

    fn op_addi(&mut self, rt: usize, rs: usize, imm: u32)
    {
        let v = (self.reg(rs) as i32).overflowing_add(imm as i32);

        self.execute_load_delay();

        if v.1 {
            self.enter_exception(Exception::Overflow);
        } else {
            self.set_reg(rt, v.0 as u32);
        }
    }

    fn op_addiu(&mut self, rt: usize, rs: usize, imm: u32)
    {
        let v = self.reg(rs).wrapping_add(imm);

        self.execute_load_delay();

        self.set_reg(rt, v);
    }

    fn op_slti(&mut self, rt: usize, rs: usize, imm: u32)
    {
        let v = ((self.reg(rs) as i32) < imm as i32) as u32;

        self.execute_load_delay();

        self.set_reg(rt, v);
    }

    fn op_sltiu(&mut self, rt: usize, rs: usize, imm: u32)
    {
        let v = (self.reg(rs) < imm) as u32;

        self.execute_load_delay();

        self.set_reg(rt, v);
    }

    fn op_andi(&mut self, rt: usize, rs: usize, imm: u32)
    {
        let v = self.reg(rs) & imm;

        self.execute_load_delay();

        self.set_reg(rt, v);
    }

    fn op_ori(&mut self, rt: usize, rs: usize, imm: u32)
    {
        let v = self.reg(rs) | imm;

        self.execute_load_delay();

        self.set_reg(rt, v);
    }

    fn op_xori(&mut self, rt: usize, rs: usize, imm: u32)
    {
        let v = self.reg(rs) ^ imm;

        self.execute_load_delay();

        self.set_reg(rt, v);
    }

    fn op_lui(&mut self, rt: usize, imm: u32)
    {
        self.execute_load_delay();

        self.set_reg(rt, imm << 16);
    }

    fn op_mfc0(&mut self, rd: usize, rt: usize)
    {
        let v = self.cop0.read(rd);

        self.execute_load_delay();

        self.ld_slot = (rt, v);
    }

    fn op_mtc0(&mut self, rd: usize, rt: usize)
    {
        let v = self.reg(rt);
        self.cop0.write(rd, v);

        self.execute_load_delay();
    }

    fn op_rfe(&mut self) {
        self.cop0.leave_exception();

        self.execute_load_delay();
    }

    fn op_lb(&mut self, rt: usize, rs: usize, offset: u32)
    {
        let addr = self.reg(rs).wrapping_add(offset);

        self.execute_load_delay();

        let v = self.load8(addr) as i8 as u32;
        self.ld_slot = (rt, v);
    }

    fn op_lh(&mut self, rt: usize, rs: usize, offset: u32)
    {
        let addr = self.reg(rs).wrapping_add(offset);

        self.execute_load_delay();

        if addr & 0x01 != 0 {
            self.enter_exception(Exception::AddrLoad);
            return;
        }

        let v = self.load16(addr) as i16 as u32;
        self.ld_slot = (rt, v);
    }

    fn op_lwl(&mut self, rt: usize, rs: usize, offset: u32)
    {
        let addr = self.reg(rs).wrapping_add(offset);

        self.execute_load_delay();

        let current = self.reg(rt);
        let aligned = self.load32(addr & 0xffff_fffc);

        let v = match addr & 0x03 {
            0 => (current & 0x00ff_ffff) | (aligned << 24),
            1 => (current & 0x0000_ffff) | (aligned << 16),
            2 => (current & 0x0000_00ff) | (aligned << 8),
            3 => (current & 0x0000_0000) | (aligned << 0),
            _ => unreachable!(),
        };

        self.ld_slot = (rt, v);
    }

    fn op_lw(&mut self, rt: usize, rs: usize, offset: u32)
    {
        let addr = self.reg(rs).wrapping_add(offset);

        self.execute_load_delay();

        if addr & 0x03 != 0 {
            self.enter_exception(Exception::AddrLoad);
            return;
        }

        let v = self.load32(addr);
        self.ld_slot = (rt, v);
    }

    fn op_lbu(&mut self, rt: usize, rs: usize, offset: u32)
    {
        let addr = self.reg(rs).wrapping_add(offset);

        self.execute_load_delay();

        let v = self.load8(addr) as u32;
        self.ld_slot = (rt, v);
    }

    fn op_lhu(&mut self, rt: usize, rs: usize, offset: u32)
    {
        let addr = self.reg(rs).wrapping_add(offset);

        self.execute_load_delay();

        let v = self.load16(addr) as u32;
        self.ld_slot = (rt, v);
    }

    fn op_lwr(&mut self, rt: usize, rs: usize, offset: u32)
    {
        let addr = self.reg(rs).wrapping_add(offset);

        self.execute_load_delay();

        let current = self.reg(rt);
        let aligned = self.load32(addr & 0xffff_fffc);

        let v = match addr & 0x03 {
            0 => (current & 0x0000_0000) | (aligned >> 0),
            1 => (current & 0xff00_0000) | (aligned >> 8),
            2 => (current & 0xffff_0000) | (aligned >> 16),
            3 => (current & 0xffff_ff00) | (aligned >> 24),
            _ => unreachable!(),
        };

        self.ld_slot = (rt, v);
    }

    fn op_sb(&mut self, rt: usize, rs: usize, offset: u32)
    {
        let addr = self.reg(rs).wrapping_add(offset);
        let v = self.reg(rt);

        self.store8(addr, v);

        self.execute_load_delay();
    }

    fn op_sh(&mut self, rt: usize, rs: usize, offset: u32)
    {
        let addr = self.reg(rs).wrapping_add(offset);
        let v = self.reg(rt);

        self.execute_load_delay();

        if addr & 0x01 != 0 {
            self.enter_exception(Exception::AddrStore);
            return;
        }

        self.store16(addr, v);
    }

    fn op_swl(&mut self, rt: usize, rs: usize, offset: u32)
    {
        let addr = self.reg(rs).wrapping_add(offset);
        let value = self.reg(rt);

        self.execute_load_delay();

        let current = self.load32(addr & 0xffff_fffc);

        let v = match addr & 0x03 {
            0 => (current & 0xffff_ff00) | (value >> 24),
            1 => (current & 0xffff_0000) | (value >> 16),
            2 => (current & 0xff00_0000) | (value >> 8),
            3 => (current & 0x0000_0000) | (value >> 0),
            _ => unreachable!(),
        };

        self.store32(addr, v);
    }

    fn op_sw(&mut self, rt: usize, rs: usize, offset: u32)
    {
        let addr = self.reg(rs).wrapping_add(offset);
        let v = self.reg(rt);

        self.execute_load_delay();

        if addr & 0x03 != 0 {
            self.enter_exception(Exception::AddrStore);
            return;
        }

        self.store32(addr, v);
    }

    fn op_swr(&mut self, rt: usize, rs: usize, offset: u32)
    {
        let addr = self.reg(rs).wrapping_add(offset);
        let value = self.reg(rt);

        self.execute_load_delay();

        let current = self.load32(addr & 0xffff_fffc);

        let v = match addr & 0x03 {
            0 => (current & 0x0000_0000) | (value << 0),
            1 => (current & 0x0000_00ff) | (value << 8),
            2 => (current & 0x0000_ffff) | (value << 16),
            3 => (current & 0x00ff_ffff) | (value << 24),
            _ => unreachable!(),
        };

        self.store32(addr, v);
    }

    fn reg(&self, index: usize) -> u32
    {
        self.regs[index]
    }

    fn set_reg(&mut self, index: usize, value: u32)
    {
        self.regs[index] = value;
        self.regs[0] = 0;
    }

    fn branch(&mut self, offset: u32)
    {
        self.branch = true;
        self.new_pc = self.pc.wrapping_add(offset << 2);
    }

    fn enter_exception(&mut self, exception: Exception) {
        let mut bd = false;
        let mut epc = self.current_pc;

        if self.branch_delay {
            bd = true;
            epc = epc.wrapping_sub(4);
        }

        self.cop0.enter_exception(epc, exception, bd);

        if self.cop0.exception_vectors() {
            self.pc = 0xbfc0_0180;
        } else {
            self.pc = 0x8000_0080;
        }

        self.new_pc = self.pc.wrapping_add(4);
    }

    fn execute_load_delay(&mut self) {
        let (reg, value) = self.ld_slot;
        self.ld_slot = (0, 0);

        self.set_reg(reg, value);
    }

    fn fetch32(&mut self) -> u32
    {
        let pc = self.pc;
        let physical_address = self.translate_address(pc);

        self.bus.load(BusWidth::WORD, physical_address)
    }

    fn translate_address(&self, virtual_address: u32) -> u32
    {
        let virtual_region = VirtualRegion::from_u32(virtual_address);
        virtual_region.translate_address(virtual_address)
    }

    fn load(&mut self, width: BusWidth, address: u32) -> u32 {
        self.last_load = address;

        if address == 0x1f80_1070 {
            return self.istat.read();
        }

        if address == 0x1f80_1074 {
            return self.imask.read();
        }

        let physical_address = self.translate_address(address);

        if !self.cop0.isolate_cache() {
            self.bus.load(width, physical_address)
        } else {
            0
        }
    }

    fn load8(&mut self, address: u32) -> u8 {
        self.load(BusWidth::BYTE, address) as u8
    }

    fn load16(&mut self, address: u32) -> u16 {
        self.load(BusWidth::HALF, address) as u16
    }

    fn load32(&mut self, address: u32) -> u32 {
        self.load(BusWidth::WORD, address)
    }

    fn store(&mut self, width: BusWidth, address: u32, value: u32) {
        self.last_store = address;

        if address == 0x1f80_1070 {
            let status = self.istat.read();
            self.istat.write(status & value);
            self.update_irq();

            return;
        }

        if address == 0x1f80_1074 {
            self.imask.write(value);
            self.update_irq();

            return;
        }

        let physical_address = self.translate_address(address);

        if !self.cop0.isolate_cache() {
            self.bus.store(width, physical_address, value);
        }
    }

    fn store8(&mut self, address: u32, value: u32) {
        self.store(BusWidth::BYTE, address, value);
    }

    fn store16(&mut self, address: u32, value: u32) {
        self.store(BusWidth::HALF, address, value);
    }

    fn store32(&mut self, address: u32, value: u32) {
        self.store(BusWidth::WORD, address, value);
    }

    pub fn debug_current_pc(&self) -> u32
    {
        self.current_pc
    }

    pub fn debug_last_load(&self) -> u32
    {
        self.last_load
    }

    pub fn debug_last_store(&self) -> u32
    {
        self.last_store
    }

    pub fn debug_register(&self, index: usize) -> u32 {
        self.regs[index]
    }

    pub fn debug_load(&self, address: u32) -> Result<u32, ()>
    {
        let physical_address = self.translate_address(address);

        self.bus.debug_load(BusWidth::WORD, physical_address)
    }
}