mod cop0;
mod dmac;
mod gte;
mod instruction;

use super::bus::{Bus, BusWidth};
use super::timekeeper::Timekeeper;

use self::cop0::{Cop0, Exception};
use self::dmac::Dmac;
use self::gte::Gte;
use self::instruction::Instruction;

#[derive(Clone, Copy)]
struct ICacheLine {
    valid: usize,
    tag: u32,
    data: [u32; 4],
}

impl ICacheLine {
    pub fn new() -> ICacheLine {
        ICacheLine {
            tag: 0xabcdabcd,
            valid: 0xcafecafe,
            data: [0xbeefbeef; 4],
        }
    }
}

struct ICache {
    lines: [ICacheLine; 256],
}

impl ICache {
    pub fn new() -> ICache {
        ICache {
            lines: [ICacheLine::new(); 256],
        }
    }
}

pub struct R3000A {
    pub pc: u32,
    pub new_pc: u32,
    current_pc: u32,

    current_instruction: u32,

    branch_delay: bool,
    branch_taken: bool,

    exception_branch_delay: bool,
    exception_branch_taken: bool,

    pub regs: [u32; 32],
    ld_slot: (usize, u32),

    hi: u32,
    lo: u32,

    icache: ICache,
    cache_control: u32,

    cop0: Cop0,
    gte: Gte,

    dmac: Dmac,
}

impl R3000A {
    pub fn new() -> R3000A {
        R3000A {
            pc: 0,
            new_pc: 0,
            current_pc: 0,

            current_instruction: 0,

            branch_delay: false,
            branch_taken: false,

            exception_branch_delay: false,
            exception_branch_taken: false,

            regs: [0; 32],
            ld_slot: (0, 0),

            hi: 0,
            lo: 0,

            icache: ICache::new(),
            cache_control: 0,

            cop0: Cop0::new(),
            gte: Gte::new(),

            dmac: Dmac::new(),
        }
    }

    pub fn get_address(&self) -> u64 {
        self as *const _ as u64
    }

    pub fn get_regs_offset(&self) -> u32 {
        let base = self.get_address();
        let reg = &self.regs as *const _ as u64;

        (reg - base) as u32
    }

    pub fn get_pc_offset(&self) -> u32 {
        let base = self.get_address();
        let pc = &self.pc as *const _ as u64;

        (pc - base) as u32
    }

    pub fn reset(&mut self) {
        self.pc = 0xbfc0_0000;
        self.new_pc = self.pc.wrapping_add(4);
        self.current_pc = self.pc;

        self.cop0.reset();
    }

    pub fn run(&mut self, bus: &mut Bus, tk: &mut Timekeeper) {
        //let mut recompiler = Recompiler::new(self);
        //let mut cs = Capstone::new()
        //    .x86()
        //    .mode(arch::x86::ArchMode::Mode64)
        //    .syntax(arch::x86::ArchSyntax::Intel)
        //    .detail(true)
        //    .build()
        //    .expect("unable to build disassembler");
//
        //let block;
//
        //if let Some(b) = recompiler.find_block(self.pc) {
        //    block = b;
        //} else {
        //    recompiler.recompile_block(bus, &self, self.pc);
        //    block = recompiler.find_block(self.pc).unwrap();
        //}
//
        //self.execute_load_delay();
//
        //let insns = cs.disasm_all(&block.code, 0).expect("unable to disassemble instructions");
//
        //for i in insns.iter() {
        //    println!("{}", i);
        //}
//
        //let mut dump = File::create(format!("0x{:08x}.bin", block.address)).unwrap();
        //dump.write(&block.code).unwrap();
//
        //unsafe {
        //    region::protect(block.code.as_ptr(), block.code.len(), Protection::ReadExecute).unwrap();
//
        //    asm!("push %rax");
        //    asm!("push %rbx");
        //    asm!("push %rcx");
        //    asm!("push %rdx");
        //    asm!("push %rsp");
        //    asm!("push %rbp");
        //    asm!("push %rsi");
        //    asm!("push %rdi");
        //    asm!("push %r8");
        //    asm!("push %r9");
        //    asm!("push %r10");
        //    asm!("push %r11");
        //    asm!("push %r12");
        //    asm!("push %r13");
        //    asm!("push %r14");
        //    asm!("push %r15");
//
        //    asm!("call $0" :: "r"(block.code.as_ptr()) :: "intel");
//
        //    asm!("pop %r15");
        //    asm!("pop %r14");
        //    asm!("pop %r13");
        //    asm!("pop %r12");
        //    asm!("pop %r11");
        //    asm!("pop %r10");
        //    asm!("pop %r9");
        //    asm!("pop %r8");
        //    asm!("pop %rdi");
        //    asm!("pop %rsi");
        //    asm!("pop %rbp");
        //    asm!("pop %rsp");
        //    asm!("pop %rdx");
        //    asm!("pop %rcx");
        //    asm!("pop %rbx");
        //    asm!("pop %rax");
//
        //    region::protect(block.code.as_ptr(), block.code.len(), Protection::ReadWrite).unwrap();
        //}
//
        //self.new_pc = self.pc + 4;
        //panic!();
        //return;

        if self.dmac.active() {
            if self.dmac.gap_started() {
                tk.sync_dmac();
            }

            if self.dmac.in_gap() {
                let cycles = tk.sync_dmac();
                self.dmac.tick_gap(cycles);

                if !self.dmac.chopping_enabled() {
                    return;
                }
            } else {
                let dma_time = self.dmac.tick(bus);
                tk.tick(dma_time as u64);
                return;
            }
        }

        self.current_pc = self.pc;
        self.exception_branch_delay = self.branch_delay;
        self.exception_branch_taken = self.branch_taken;

        tk.tick(1);

        let cop0_break = self.cop0.test_code(self.pc);

        if self.pc & 0x3 != 0 {
            self.cop0.set_bad_vaddr(self.current_pc);
            self.enter_exception(Exception::AddrLoad);

            self.execute_load_delay();

            return;
        }

        self.update_irq(bus);

        let iec = self.cop0.iec();
        let im = self.cop0.im();

        let (ins, err) = self.fetch32(bus, tk);

        self.current_instruction = ins;
        let instruction = Instruction(ins);

        if iec && im {
            self.enter_exception(Exception::Interrupt);

            if (ins >> 25) == 0x25 {
                self.op_cop2_command(instruction.target());
            }

            self.execute_load_delay();
            return;
        }

        if cop0_break {
            self.cop0_break();

            self.execute_load_delay();
            return;
        }

        self.branch_delay = false;
        self.branch_taken = false;

        if err {
            self.enter_exception(Exception::IBusError);

            self.execute_load_delay();
            return;
        }

        self.pc = self.new_pc;
        self.new_pc += 4;

        if ins == 0 {
            self.execute_load_delay();
            return;
        }

        self.execute(bus, tk, instruction);
    }

    fn update_irq(&mut self, bus: &mut Bus) {
        if bus.intc().pending() {
            self.cop0.set_interrupt_bit();
        } else {
            self.cop0.clear_interrupt_bit();
        }
    }

    fn execute(&mut self,
               bus: &mut Bus,
               tk: &mut Timekeeper,
               i: Instruction) {
        match i.opcode() {
            0x00 => self.op_special(i),
            0x01 => self.op_bcond(i.rs(), i.rt(), i.imm_se()),
            0x02 => self.op_j(i.target()),
            0x03 => self.op_jal(i.target()),
            0x04 => self.op_beq(i.rs(), i.rt(), i.imm_se()),
            0x05 => self.op_bne(i.rs(), i.rt(), i.imm_se()),
            0x06 => self.op_blez(i.rs(), i.imm_se()),
            0x07 => self.op_bgtz(i.rs(), i.imm_se()),
            0x08 => self.op_addi(i.rt(), i.rs(), i.imm_se()),
            0x09 => self.op_addiu(i.rt(), i.rs(), i.imm_se()),
            0x0a => self.op_slti(i.rt(), i.rs(), i.imm_se()),
            0x0b => self.op_sltiu(i.rt(), i.rs(), i.imm_se()),
            0x0c => self.op_andi(i.rt(), i.rs(), i.imm()),
            0x0d => self.op_ori(i.rt(), i.rs(), i.imm()),
            0x0e => self.op_xori(i.rt(), i.rs(), i.imm()),
            0x0f => self.op_lui(i.rt(), i.imm()),
            0x10 => self.op_cop0(i),
            0x11 => (),
            0x12 => self.op_cop2(i),
            0x13 => (),
            0x20 => self.op_lb(bus, tk, i.rt(), i.rs(), i.imm_se()),
            0x21 => self.op_lh(bus, tk, i.rt(), i.rs(), i.imm_se()),
            0x22 => self.op_lwl(bus, tk, i.rt(), i.rs(), i.imm_se()),
            0x23 => self.op_lw(bus, tk, i.rt(), i.rs(), i.imm_se()),
            0x24 => self.op_lbu(bus, tk, i.rt(), i.rs(), i.imm_se()),
            0x25 => self.op_lhu(bus, tk, i.rt(), i.rs(), i.imm_se()),
            0x26 => self.op_lwr(bus, tk, i.rt(), i.rs(), i.imm_se()),
            0x28 => self.op_sb(bus, tk, i.rt(), i.rs(), i.imm_se()),
            0x29 => self.op_sh(bus, tk, i.rt(), i.rs(), i.imm_se()),
            0x2a => self.op_swl(bus, tk, i.rt(), i.rs(), i.imm_se()),
            0x2b => self.op_sw(bus, tk, i.rt(), i.rs(), i.imm_se()),
            0x2e => self.op_swr(bus, tk, i.rt(), i.rs(), i.imm_se()),
            0x30 => self.op_lwcx(bus, tk, i.rt(), i.rs(), i.imm_se()),
            0x31 => self.op_lwcx(bus, tk, i.rt(), i.rs(), i.imm_se()),
            0x32 => self.op_lwc2(bus, tk, i.rt(), i.rs(), i.imm_se()),
            0x33 => self.op_lwcx(bus, tk, i.rt(), i.rs(), i.imm_se()),
            0x38 => self.op_swcx(bus, tk, i.rt(), i.rs(), i.imm_se()),
            0x39 => self.op_swcx(bus, tk, i.rt(), i.rs(), i.imm_se()),
            0x3a => self.op_swc2(bus, tk, i.rt(), i.rs(), i.imm_se()),
            0x3b => self.op_swcx(bus, tk, i.rt(), i.rs(), i.imm_se()),
            _ => self.op_illegal(),
        }
    }

    fn op_special(&mut self, i: Instruction) {
        match i.function() {
            0x00 => self.op_sll(i.rd(), i.rt(), i.shift()),
            0x02 => self.op_srl(i.rd(), i.rt(), i.shift()),
            0x03 => self.op_sra(i.rd(), i.rt(), i.shift()),
            0x04 => self.op_sllv(i.rd(), i.rt(), i.rs()),
            0x06 => self.op_srlv(i.rd(), i.rt(), i.rs()),
            0x07 => self.op_srav(i.rd(), i.rt(), i.rs()),
            0x08 => self.op_jr(i.rs()),
            0x09 => self.op_jalr(i.rd(), i.rs()),
            0x0c => self.op_syscall(),
            0x0d => self.op_break(),
            0x10 => self.op_mfhi(i.rd()),
            0x11 => self.op_mthi(i.rs()),
            0x12 => self.op_mflo(i.rd()),
            0x13 => self.op_mtlo(i.rs()),
            0x18 => self.op_mult(i.rs(), i.rt()),
            0x19 => self.op_multu(i.rs(), i.rt()),
            0x1a => self.op_div(i.rs(), i.rt()),
            0x1b => self.op_divu(i.rs(), i.rt()),
            0x20 => self.op_add(i.rd(), i.rs(), i.rt()),
            0x21 => self.op_addu(i.rd(), i.rs(), i.rt()),
            0x22 => self.op_sub(i.rd(), i.rs(), i.rt()),
            0x23 => self.op_subu(i.rd(), i.rs(), i.rt()),
            0x24 => self.op_and(i.rd(), i.rs(), i.rt()),
            0x25 => self.op_or(i.rd(), i.rs(), i.rt()),
            0x26 => self.op_xor(i.rd(), i.rs(), i.rt()),
            0x27 => self.op_nor(i.rd(), i.rs(), i.rt()),
            0x2a => self.op_slt(i.rd(), i.rs(), i.rt()),
            0x2b => self.op_sltu(i.rd(), i.rs(), i.rt()),
            _ => self.op_illegal(),
        };
    }

    fn op_sll(&mut self, rd: usize, rt: usize, shift: usize) {
        let v = self.reg(rt) << shift;

        self.execute_load_delay();

        self.set_reg(rd, v);
    }

    fn op_srl(&mut self, rd: usize, rt: usize, shift: usize) {
        let v = self.reg(rt) >> shift;

        self.execute_load_delay();

        self.set_reg(rd, v);
    }

    fn op_sra(&mut self, rd: usize, rt: usize, shift: usize) {
        let v = (self.reg(rt) as i32 >> shift) as u32;

        self.execute_load_delay();

        self.set_reg(rd, v);
    }

    fn op_sllv(&mut self, rd: usize, rt: usize, rs: usize) {
        let v = self.reg(rt) << self.reg(rs);

        self.execute_load_delay();

        self.set_reg(rd, v);
    }

    fn op_srlv(&mut self, rd: usize, rt: usize, rs: usize) {
        let v = self.reg(rt) >> self.reg(rs);

        self.execute_load_delay();

        self.set_reg(rd, v);
    }

    fn op_srav(&mut self, rd: usize, rt: usize, rs: usize) {
        let v = (self.reg(rt) as i32 >> self.reg(rs)) as u32;

        self.execute_load_delay();

        self.set_reg(rd, v);
    }

    fn op_jr(&mut self, rs: usize) {
        self.new_pc = self.reg(rs);

        self.branch_delay = true;
        self.branch_taken = true;

        self.execute_load_delay();
    }

    fn op_jalr(&mut self, rd: usize, rs: usize) {
        let ra = self.new_pc;

        self.branch_delay = true;
        self.branch_taken = true;
        self.new_pc = self.reg(rs);

        self.execute_load_delay();

        self.set_reg(rd, ra);
    }

    fn op_syscall(&mut self) {
        self.execute_load_delay();

        self.enter_exception(Exception::Syscall);
    }

    fn op_break(&mut self) {
        self.execute_load_delay();

        self.enter_exception(Exception::Breakpoint);
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

    fn op_mult(&mut self, rs: usize, rt: usize) {
        let m1 = (self.reg(rs) as i32) as i64;
        let m2 = (self.reg(rt) as i32) as i64;

        let r = (m1 * m2) as u64;

        self.hi = (r >> 32) as u32;
        self.lo = r as u32;

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

    fn op_add(&mut self, rd: usize, rs: usize, rt: usize) {
        let v = (self.reg(rs) as i32).overflowing_add(self.reg(rt) as i32);

        self.execute_load_delay();

        if v.1 {
            self.enter_exception(Exception::Overflow);
        } else {
            self.set_reg(rd, v.0 as u32);
        }
    }

    fn op_addu(&mut self, rd: usize, rs: usize, rt: usize) {
        let v = self.reg(rs).wrapping_add(self.reg(rt));

        self.execute_load_delay();

        self.set_reg(rd, v);
    }

    fn op_sub(&mut self, rd: usize, rs: usize, rt: usize) {
        let v = (self.reg(rs) as i32).overflowing_sub(self.reg(rt) as i32);

        self.execute_load_delay();

        if v.1 {
            self.enter_exception(Exception::Overflow);
        } else {
            self.set_reg(rd, v.0 as u32);
        }
    }

    fn op_subu(&mut self, rd: usize, rs: usize, rt: usize) {
        let v = self.reg(rs).wrapping_sub(self.reg(rt));

        self.execute_load_delay();

        self.set_reg(rd, v);
    }

    fn op_and(&mut self, rd: usize, rs: usize, rt: usize) {
        let v = self.reg(rs) & self.reg(rt);

        self.execute_load_delay();

        self.set_reg(rd, v);
    }

    fn op_or(&mut self, rd: usize, rs: usize, rt: usize) {
        let v = self.reg(rs) | self.reg(rt);

        self.execute_load_delay();

        self.set_reg(rd, v);
    }

    fn op_xor(&mut self, rd: usize, rs: usize, rt: usize) {
        let v = self.reg(rs) ^ self.reg(rt);

        self.execute_load_delay();

        self.set_reg(rd, v);
    }

    fn op_nor(&mut self, rd: usize, rs: usize, rt: usize) {
        let v = !(self.reg(rs) | self.reg(rt));

        self.execute_load_delay();

        self.set_reg(rd, v);
    }

    fn op_slt(&mut self, rd: usize, rs: usize, rt: usize) {
        let v = ((self.reg(rs) as i32) < (self.reg(rt) as i32)) as u32;

        self.execute_load_delay();

        self.set_reg(rd, v);
    }

    fn op_sltu(&mut self, rd: usize, rs: usize, rt: usize) {
        let v = (self.reg(rs) < self.reg(rt)) as u32;

        self.execute_load_delay();

        self.set_reg(rd, v);
    }

    fn op_bcond(&mut self, rs: usize, rt: usize, offset: u32) {
        let s = self.reg(rs) as i32;

        let result = (s ^ ((rt as i32) << 31)) < 0;
        let link = (rt & 0x1e) == 0x10;

        self.execute_load_delay();

        if link {
            let ra = self.new_pc;
            self.set_reg(31, ra);
        }

        self.branch_delay = true;

        if result {
            self.branch(offset);
        }
    }

    fn op_j(&mut self, target: u32) {
        self.execute_load_delay();

        self.branch_delay = true;
        self.branch_taken = true;
        self.new_pc = (self.pc & 0xf000_0000) | (target << 2);
    }

    fn op_jal(&mut self, target: u32) {
        self.execute_load_delay();

        let ra = self.new_pc;

        self.branch_delay = true;
        self.branch_taken = true;
        self.new_pc = (self.pc & 0xf000_0000) | (target << 2);

        self.set_reg(31, ra);
    }

    fn op_beq(&mut self, rs: usize, rt: usize, offset: u32) {
        if self.reg(rs) == self.reg(rt) {
            self.branch(offset);
        }

        self.branch_delay = true;

        self.execute_load_delay();
    }

    fn op_bne(&mut self, rs: usize, rt: usize, offset: u32) {
        if self.reg(rs) != self.reg(rt) {
            self.branch(offset);
        }

        self.branch_delay = true;

        self.execute_load_delay();
    }

    fn op_blez(&mut self, rs: usize, offset: u32) {
        if self.reg(rs) as i32 <= 0 {
            self.branch(offset);
        }

        self.branch_delay = true;

        self.execute_load_delay();
    }

    fn op_bgtz(&mut self, rs: usize, offset: u32) {
        if self.reg(rs) as i32 > 0 {
            self.branch(offset);
        }

        self.branch_delay = true;

        self.execute_load_delay();
    }

    fn op_addi(&mut self, rt: usize, rs: usize, imm: u32) {
        let v = (self.reg(rs) as i32).overflowing_add(imm as i32);

        self.execute_load_delay();

        if v.1 {
            self.enter_exception(Exception::Overflow);
        } else {
            self.set_reg(rt, v.0 as u32);
        }
    }

    fn op_addiu(&mut self, rt: usize, rs: usize, imm: u32) {
        let v = self.reg(rs).wrapping_add(imm);

        self.execute_load_delay();

        self.set_reg(rt, v);
    }

    fn op_slti(&mut self, rt: usize, rs: usize, imm: u32) {
        let v = ((self.reg(rs) as i32) < imm as i32) as u32;

        self.execute_load_delay();

        self.set_reg(rt, v);
    }

    fn op_sltiu(&mut self, rt: usize, rs: usize, imm: u32) {
        let v = (self.reg(rs) < imm) as u32;

        self.execute_load_delay();

        self.set_reg(rt, v);
    }

    fn op_andi(&mut self, rt: usize, rs: usize, imm: u32) {
        let v = self.reg(rs) & imm;

        self.execute_load_delay();

        self.set_reg(rt, v);
    }

    fn op_ori(&mut self, rt: usize, rs: usize, imm: u32) {
        let v = self.reg(rs) | imm;

        self.execute_load_delay();

        self.set_reg(rt, v);
    }

    fn op_xori(&mut self, rt: usize, rs: usize, imm: u32) {
        let v = self.reg(rs) ^ imm;

        self.execute_load_delay();

        self.set_reg(rt, v);
    }

    fn op_lui(&mut self, rt: usize, imm: u32) {
        self.execute_load_delay();

        self.set_reg(rt, imm << 16);
    }

    fn op_cop0(&mut self, i: Instruction) {
        match i.rs() {
            0x00 => self.op_mfc0(i.rd(), i.rt()),
            0x04 => self.op_mtc0(i.rd(), i.rt()),
            0x10 => self.op_rfe(),
            _ => panic!("[CPU] [ERROR] Unrecognised instruction 0x{:08x}", i.0),
        };
    }

    fn op_mfc0(&mut self, rd: usize, rt: usize) {
        if rd == 0 || rd == 2 || rd == 4 || rd == 10 {
            self.enter_exception(Exception::Reserved);
            return;
        }

        let v = self.cop0.read(rd);

        self.update_load_delay(rt, v);
    }

    fn op_mtc0(&mut self, rd: usize, rt: usize) {
        let v = self.reg(rt);

        let prev_iec = self.cop0.iec();

        self.cop0.write(rd, v);

        self.execute_load_delay();

        if !prev_iec && self.cop0.iec() && self.cop0.im() {
            self.pc = self.new_pc;
            self.enter_exception(Exception::Interrupt);
        }
    }

    fn op_rfe(&mut self) {
        self.execute_load_delay();

        self.cop0.leave_exception();
    }

    fn op_cop2(&mut self, i: Instruction) {
        match i.rs() & 0x10 {
            0x00 => match i.rs() & 0x0f {
                0x00 => self.op_mfc2(i.rd(), i.rt()),
                0x02 => self.op_cfc2(i.rd(), i.rt()),
                0x04 => self.op_mtc2(i.rd(), i.rt()),
                0x06 => self.op_ctc2(i.rd(), i.rt()),
                _ => panic!("[CPU] [ERROR] Unrecognised instruction 0x{:08x}", i.0),
            },
            0x10 => self.op_cop2_command(i.target()),
            _ => unreachable!(),
        };
    }

    fn op_mfc2(&mut self, rd: usize, rt: usize) {
        let v = self.gte.read_data(rd);

        self.update_load_delay(rt, v);
    }

    fn op_cfc2(&mut self, rd: usize, rt: usize) {
        let v = self.gte.read_control(rd);

        self.update_load_delay(rt, v);
    }

    fn op_mtc2(&mut self, rd: usize, rt: usize) {
        let v = self.reg(rt);
        self.gte.write_data(rd, v);

        self.execute_load_delay();
    }

    fn op_ctc2(&mut self, rd: usize, rt: usize) {
        let v = self.reg(rt);
        self.gte.write_control(rd, v);

        self.execute_load_delay();
    }

    fn op_cop2_command(&mut self, function: u32) {
        self.gte.execute(function);

        self.execute_load_delay();
    }

    fn op_lb(&mut self,
             bus: &mut Bus,
             tk: &mut Timekeeper,
             rt: usize,
             rs: usize,
             offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);

        if self.cop0.test_read(addr) {
            self.cop0_break();

            self.execute_load_delay();
            return;
        }

        let (v, err) = self.load8(bus, tk, addr);

        if err {
            self.enter_exception(Exception::DBusError);

            self.execute_load_delay();
            return;
        }

        self.update_load_delay(rt, v as i8 as u32);
    }

    fn op_lh(&mut self,
             bus: &mut Bus,
             tk: &mut Timekeeper,
             rt: usize,
             rs: usize,
             offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);

        let cop0_break = self.cop0.test_read(addr);

        if addr & 0x01 != 0 {
            self.cop0.set_bad_vaddr(addr);
            self.enter_exception(Exception::AddrLoad);
            return;
        }

        if cop0_break {
            self.cop0_break();

            self.execute_load_delay();
            return;
        }

        let (v, err) = self.load16(bus, tk, addr);

        if err {
            self.enter_exception(Exception::DBusError);

            self.execute_load_delay();
            return;
        }

        self.update_load_delay(rt, v as i16 as u32);
    }

    fn op_lwl(&mut self,
              bus: &mut Bus,
              tk: &mut Timekeeper,
              rt: usize,
              rs: usize,
              offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);

        if self.cop0.test_read(addr & 0xffff_fffc) {
            self.cop0_break();

            self.execute_load_delay();
            return;
        }

        let mut current = self.reg(rt);
        let (aligned, err) = self.load32(bus, tk, addr & 0xffff_fffc);

        if err {
            self.enter_exception(Exception::DBusError);

            self.execute_load_delay();
            return;
        }

        if self.ld_slot.0 == rt {
            current = self.ld_slot.1;
        } else {
            self.execute_load_delay();
        }

        self.ld_slot.0 = rt;

        self.ld_slot.1 = match addr & 0x03 {
            0 => (current & 0x00ff_ffff) | (aligned << 24),
            1 => (current & 0x0000_ffff) | (aligned << 16),
            2 => (current & 0x0000_00ff) | (aligned << 8),
            3 => aligned + 1,
            _ => unreachable!(),
        };
    }

    fn op_lw(&mut self,
             bus: &mut Bus,
             tk: &mut Timekeeper,
             rt: usize,
             rs: usize,
             offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);

        let cop0_break = self.cop0.test_read(addr);

        if addr & 0x03 != 0 {
            self.cop0.set_bad_vaddr(addr);
            self.enter_exception(Exception::AddrLoad);
            return;
        }

        if cop0_break {
            self.cop0_break();

            self.execute_load_delay();
            return;
        }

        let (v, err) = self.load32(bus, tk, addr);

        if err {
            self.enter_exception(Exception::DBusError);

            self.execute_load_delay();
            return;
        }

        self.update_load_delay(rt, v);
    }

    fn op_lbu(&mut self,
              bus: &mut Bus,
              tk: &mut Timekeeper,
              rt: usize,
              rs: usize,
              offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);

        if self.cop0.test_read(addr) {
            self.cop0_break();

            self.execute_load_delay();
            return;
        }

        let (v, err) = self.load8(bus, tk, addr);

        if err {
            self.enter_exception(Exception::DBusError);

            self.execute_load_delay();
            return;
        }

        self.update_load_delay(rt, v as u32);
    }

    fn op_lhu(&mut self,
              bus: &mut Bus,
              tk: &mut Timekeeper,
              rt: usize,
              rs: usize,
              offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);

        let cop0_break = self.cop0.test_read(addr);

        if addr & 0x01 != 0 {
            self.cop0.set_bad_vaddr(addr);
            self.enter_exception(Exception::AddrLoad);
            return;
        }

        if cop0_break {
            self.cop0_break();

            self.execute_load_delay();
            return;
        }

        let (v, err) = self.load16(bus, tk, addr);

        if err {
            self.enter_exception(Exception::DBusError);

            self.execute_load_delay();
            return;
        }

        self.update_load_delay(rt, v as u32);
    }

    fn op_lwr(&mut self,
              bus: &mut Bus,
              tk: &mut Timekeeper,
              rt: usize,
              rs: usize,
              offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);

        if self.cop0.test_read(addr) {
            self.cop0_break();

            self.execute_load_delay();
            return;
        }

        let mut current = self.reg(rt);
        let (aligned, err) = self.load32(bus, tk, addr & 0xffff_fffc);

        if err {
            self.enter_exception(Exception::DBusError);

            self.execute_load_delay();
            return;
        }

        if self.ld_slot.0 == rt {
            current = self.ld_slot.1;
        } else {
            self.execute_load_delay();
        }

        self.ld_slot.0 = rt;

        self.ld_slot.1 = match addr & 0x03 {
            0 => aligned,
            1 => (current & 0xff00_0000) | (aligned >> 8),
            2 => (current & 0xffff_0000) | (aligned >> 16),
            3 => (current & 0xffff_ff00) | (aligned >> 24),
            _ => unreachable!(),
        };
    }

    fn op_sb(&mut self,
             bus: &mut Bus,
             tk: &mut Timekeeper,
             rt: usize,
             rs: usize,
             offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);
        let v = self.reg(rt);

        self.execute_load_delay();

        if self.cop0.test_write(addr) {
            self.cop0_break();
            return;
        }

        let err = self.store8(bus, tk, addr, v);

        if err {
            self.enter_exception(Exception::DBusError);
        }
    }

    fn op_sh(&mut self,
             bus: &mut Bus,
             tk: &mut Timekeeper,
             rt: usize,
             rs: usize,
             offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);
        let v = self.reg(rt);

        self.execute_load_delay();
        
        let cop0_break = self.cop0.test_write(addr);

        if addr & 0x01 != 0 {
            self.cop0.set_bad_vaddr(addr);
            self.enter_exception(Exception::AddrStore);
            return;
        }

        if cop0_break {
            self.cop0_break();
            return;
        }

        let err = self.store16(bus, tk, addr, v);

        if err {
            self.enter_exception(Exception::DBusError);
        }
    }

    fn op_swl(&mut self,
              bus: &mut Bus,
              tk: &mut Timekeeper,
              rt: usize,
              rs: usize,
              offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);

        if self.cop0.test_write(addr & 0xffff_fffc) {
            self.cop0_break();

            self.execute_load_delay();
            return;
        }

        let value = self.reg(rt);

        let (current, err) = self.load32(bus, tk, addr & 0xffff_fffc);

        if err {
            self.enter_exception(Exception::DBusError);

            self.execute_load_delay();
            return;
        }

        let v = match addr & 0x03 {
            0 => (current & 0xffff_ff00) | (value >> 24),
            1 => (current & 0xffff_0000) | (value >> 16),
            2 => (current & 0xff00_0000) | (value >> 8),
            3 => value,
            _ => unreachable!(),
        };

        let err = self.store32(bus, tk, addr & 0xffff_fffc, v);

        if err {
            self.enter_exception(Exception::DBusError);
        }

        self.execute_load_delay();
    }

    fn op_sw(&mut self,
             bus: &mut Bus,
             tk: &mut Timekeeper,
             rt: usize,
             rs: usize,
             offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);
        let v = self.reg(rt);

        self.execute_load_delay();

        let cop0_break = self.cop0.test_write(addr);

        if addr & 0x03 != 0 {
            self.cop0.set_bad_vaddr(addr);
            self.enter_exception(Exception::AddrStore);
            return;
        }

        if cop0_break {
            self.cop0_break();
            return;
        }

        let err = self.store32(bus, tk, addr, v);

        if err {
            self.enter_exception(Exception::DBusError);
        }
    }

    fn op_swr(&mut self,
              bus: &mut Bus,
              tk: &mut Timekeeper,
              rt: usize,
              rs: usize,
              offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);

        let value = self.reg(rt);

        if self.cop0.test_write(addr) {
            self.cop0_break();

            self.execute_load_delay();
            return;
        }

        let (current, err) = self.load32(bus, tk, addr & 0xffff_fffc);

        if err {
            self.enter_exception(Exception::DBusError);

            self.execute_load_delay();
            return;
        }

        let v = match addr & 0x03 {
            0 => value,
            1 => (current & 0x0000_00ff) | (value << 8),
            2 => (current & 0x0000_ffff) | (value << 16),
            3 => (current & 0x00ff_ffff) | (value << 24),
            _ => unreachable!(),
        };

        let err = self.store32(bus, tk, addr & 0xffff_fffc, v);

        if err {
            self.enter_exception(Exception::DBusError);
            return;
        }

        self.execute_load_delay();
    }

    fn op_lwcx(&mut self,
               bus: &mut Bus,
               tk: &mut Timekeeper,
               _: usize,
               rs: usize,
               offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);

        self.execute_load_delay();

        let cop0_break = self.cop0.test_read(addr);

        if addr & 0x03 != 0 {
            self.cop0.set_bad_vaddr(addr);
            self.enter_exception(Exception::AddrLoad);
            return;
        }

        if cop0_break {
            self.cop0_break();
            return;
        }

        let (_, err) = self.load32(bus, tk, addr);

        if err {
            self.enter_exception(Exception::DBusError);
            return;
        }
    }

    fn op_lwc2(&mut self,
               bus: &mut Bus,
               tk: &mut Timekeeper,
               rt: usize,
               rs: usize,
               offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);

        self.execute_load_delay();

        let cop0_break = self.cop0.test_read(addr);

        if addr & 0x03 != 0 {
            self.cop0.set_bad_vaddr(addr);
            self.enter_exception(Exception::AddrLoad);
            return;
        }

        if cop0_break {
            self.cop0_break();
            return;
        }

        let (v, err) = self.load32(bus, tk, addr);

        if err {
            self.enter_exception(Exception::DBusError);
            return;
        }

        self.gte.write_data(rt, v);
    }

    fn op_swcx(&mut self,
               bus: &mut Bus,
               tk: &mut Timekeeper,
               _: usize,
               rs: usize,
               offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);

        self.execute_load_delay();

        let cop0_break = self.cop0.test_write(addr);

        if addr & 0x03 != 0 {
            self.cop0.set_bad_vaddr(addr);
            self.enter_exception(Exception::AddrStore);
            return;
        }

        if cop0_break {
            self.cop0_break();
            return;
        }

        let err = self.store32(bus, tk, addr, 0);

        if err {
            self.enter_exception(Exception::DBusError);
            return;
        }
    }

    fn op_swc2(&mut self,
               bus: &mut Bus,
               tk: &mut Timekeeper,
               rt: usize,
               rs: usize,
               offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);
        let v = self.gte.read_data(rt);

        self.execute_load_delay();

        let cop0_break = self.cop0.test_write(addr);

        if addr & 0x03 != 0 {
            self.cop0.set_bad_vaddr(addr);
            self.enter_exception(Exception::AddrStore);
            return;
        }

        if cop0_break {
            self.cop0_break();
            return;
        }

        let err = self.store32(bus, tk, addr, v);

        if err {
            self.enter_exception(Exception::DBusError);
            return;
        }
    }

    fn op_illegal(&mut self) {
        self.execute_load_delay();

        self.enter_exception(Exception::Reserved);
    }

    fn reg(&self, index: usize) -> u32 {
        unsafe {
            *self.regs.get_unchecked(index)
        }
    }

    fn set_reg(&mut self, index: usize, value: u32) {
        unsafe {
            *self.regs.get_unchecked_mut(index) = value;
            *self.regs.get_unchecked_mut(0) = 0;
        }
    }

    fn branch(&mut self, offset: u32) {
        self.branch_taken = true;
        self.new_pc = self.pc.wrapping_add(offset << 2);
    }

    fn enter_exception(&mut self, exception: Exception) {
        let bd = self.exception_branch_delay;
        let bt = self.exception_branch_taken;

        let cop = match (exception == Exception::IBusError)
                        || (exception == Exception::DBusError)
                        || (exception == Exception::Breakpoint) {
            true => 0,
            false => (self.current_instruction >> 26) & 0x3,
        } as u8;

        let mut epc = match exception == Exception::Interrupt {
            true => self.pc,
            false => self.current_pc,
        };

        if bd {
            epc = epc.wrapping_sub(4);
            self.cop0.set_jumpdest(self.pc);
        }

        if (exception != Exception::Interrupt)
           && (exception != Exception::Syscall) {
               println!("[CPU] [WARN] Unexpected exception: {:#?}", exception);
           }

        self.cop0.enter_exception(epc, exception, bd, bt, cop);

        if self.cop0.exception_vectors() {
            self.pc = 0xbfc0_0180;
        } else {
            self.pc = 0x8000_0080;
        }

        self.new_pc = self.pc.wrapping_add(4);
    }

    fn cop0_break(&mut self) {
        self.enter_exception(Exception::Breakpoint);

        if self.cop0.exception_vectors() {
            self.pc = 0xbfc0_0140;
        } else {
            self.pc = 0x8000_0040;
        }

        self.new_pc = self.pc.wrapping_add(4);
    }

    fn execute_load_delay(&mut self) {
        let (reg, value) = self.ld_slot;
        self.ld_slot = (0, 0);

        self.set_reg(reg, value);
    }

    fn update_load_delay(&mut self, index: usize, value: u32) {
        if self.ld_slot.0 != index {
            self.execute_load_delay();
        }

        self.ld_slot = (index, value);
    }

    fn fetch32(&mut self,
               bus: &mut Bus,
               tk: &mut Timekeeper) -> (u32, bool) {
        let pc = self.pc;
        let physical_address = R3000A::translate_address(pc);

        if self.icache_enabled() && (pc < 0xa0000000) {
            /* Not sure if top bit is part of tag, only two options anyway */
            let tag = pc & 0x7ffff000;

            let line = ((pc & 0xff0) >> 4) as usize;
            let index = ((pc & 0xc) >> 2) as usize; 

            let cline = unsafe { self.icache.lines.get_unchecked_mut(line) };

            if (cline.tag != tag) || (cline.valid > index) {
                let mut address = (physical_address & !0xf) + (0x4 * index as u32);

                for i in index..4 {
                    let data = unsafe { bus.load(tk, BusWidth::WORD, address) };

                    if data.1 {
                        return (0, true);
                    }

                    unsafe { *cline.data.get_unchecked_mut(i) = data.0 };
                    address += 4;
                }

                cline.tag = tag;
                cline.valid = index;

                tk.tick(5);
            }

            return (unsafe { *cline.data.get_unchecked(index) }, false);
        }

        tk.tick(5);
        unsafe { bus.load(tk, BusWidth::WORD, physical_address) }
    }

    pub fn translate_address(virtual_address: u32) -> u32 {
        match virtual_address >> 29 {
            0b000..=0b011 => virtual_address,
            0b100 => virtual_address & 0x7fff_ffff,
            0b101 => virtual_address & 0x1fff_ffff,
            0b110..=0b111 => virtual_address,
            _ => unreachable!(),
        }
    }

    fn icache_enabled(&self) -> bool {
        (self.cache_control & 0x800) != 0
    }

    fn icache_tag_test(&self) -> bool {
        (self.cache_control & 0x4) != 0
    }

    fn load(&mut self,
            bus: &mut Bus,
            tk: &mut Timekeeper,
            width: BusWidth, address: u32) -> (u32, bool) {
        let physical_address = R3000A::translate_address(address);

        if self.cop0.isolate_cache() {
            let line = ((address & 0xff0) >> 4) as usize;
            let index = ((address & 0xc) >> 2) as usize; 

            let cline = unsafe { self.icache.lines.get_unchecked(line) };

            if self.icache_tag_test() {
                return (cline.tag, false);
            } else {
                return (unsafe { *cline.data.get_unchecked(index) }, false);
            }
        }

        tk.tick(5);

        match physical_address {
            0x1f80_1080..=0x1f80_10ff => (self.dmac.read(address), false),
            0xfffe_0130 => (self.cache_control, false),
            _ => unsafe { bus.load(tk, width, physical_address) },
        }
    }

    fn load8(&mut self,
             bus: &mut Bus,
             tk: &mut Timekeeper,
             address: u32) -> (u8, bool) {
        let (val, error) =  self.load(bus, tk, BusWidth::BYTE, address);

        (val as u8, error)
    }

    fn load16(&mut self,
              bus: &mut Bus,
              tk: &mut Timekeeper,
              address: u32) -> (u16, bool) {
        let (val, error) = self.load(bus, tk, BusWidth::HALF, address);

        (val as u16, error)
    }

    fn load32(&mut self,
              bus: &mut Bus,
              tk: &mut Timekeeper,
              address: u32) -> (u32, bool) {
        self.load(bus, tk, BusWidth::WORD, address)
    }

    fn store(&mut self,
             bus: &mut Bus,
             tk: &mut Timekeeper,
             width: BusWidth, address: u32, value: u32) -> bool {
        let physical_address = R3000A::translate_address(address);

        if self.cop0.isolate_cache() {
            let line = ((address & 0xff0) >> 4) as usize;
            let index = ((address & 0xc) >> 2) as usize; 

            let tag_test = self.icache_tag_test();

            let cline = unsafe { self.icache.lines.get_unchecked_mut(line) };

            if tag_test {
                cline.tag = value;
            } else {
                unsafe { *cline.data.get_unchecked_mut(index) = value };
            }

            cline.valid = 4;

            return false;
        }

        tk.tick(5);

        match physical_address {
            0x1f80_1080..=0x1f80_10ff => {
                self.dmac.write(bus.intc(), address, value);
                false
            },
            0xfffe_0130 => {
                self.cache_control = value;
                false
            },
            _ => unsafe { bus.store(tk, width, physical_address, value) },
        }
    }

    fn store8(&mut self,
              bus: &mut Bus,
              tk: &mut Timekeeper,
              address: u32,
              value: u32) -> bool {
        self.store(bus, tk, BusWidth::BYTE, address, value)
    }

    fn store16(&mut self,
               bus: &mut Bus,
               tk: &mut Timekeeper,
               address: u32,
               value: u32) -> bool {
        self.store(bus, tk, BusWidth::HALF, address, value)
    }

    fn store32(&mut self,
               bus: &mut Bus,
               tk: &mut Timekeeper,
               address: u32,
               value: u32) -> bool {
        self.store(bus, tk, BusWidth::WORD, address, value)
    }
}
