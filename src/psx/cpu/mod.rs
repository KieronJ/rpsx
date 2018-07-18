mod cop0;
mod gte;
pub mod ops;

use util;

use super::bus::{Bus, BusWidth};

use self::cop0::{Cop0, Exception};
use self::gte::Gte;
use self::ops::Operation;

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
    gte: Gte,

    bus: Bus,
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
            gte: Gte::new(),

            bus: bus,
        }
    }

    pub fn bus(&mut self) -> &mut Bus {
        &mut self.bus
    }

    pub fn reset(&mut self) {
        self.cop0.reset(self.current_pc);

        self.pc = 0xbfc0_0000;
        self.new_pc = self.pc.wrapping_add(4);
        self.current_pc = self.pc;
    }

    pub fn run(&mut self) {
        self.branch_delay = self.branch;
        self.branch = false;

        self.current_pc = self.pc;

        if self.pc & 0x03 != 0 {
            self.cop0.set_bad_vaddr(self.pc);
            self.enter_exception(Exception::AddrLoad);
            return;
        }

        self.last_load = 0;
        self.last_store = 0;

        let op: Operation = self.fetch32().into();

        self.pc = self.new_pc;
        self.new_pc = self.pc.wrapping_add(4);

        self.update_irq();

        let iec = self.cop0.iec();
        let im = self.cop0.im();

        if iec && im {
            self.enter_exception(Exception::Interrupt);

            if let Operation::Cop2(_) = op {
                self.execute(op);
            }

            return;
        }

        //if self.current_pc == 0xa0 {
        //    let function = self.regs[9];
        //    let arg1 = self.regs[4];
        //    let arg2 = self.regs[5];
        //    let arg3 = self.regs[6];
        //
        //    self.disassemble_bios_call_a0(function, arg1, arg2, arg3);
        //}

        //if self.current_pc == 0xb0 {
        //    let function = self.regs[9];
        //    let arg1 = self.regs[4];
        //    let arg2 = self.regs[5];
        //    let arg3 = self.regs[6];
        //    let arg4 = self.regs[7];
        //
        //    self.disassemble_bios_call_b0(function, arg1, arg2, arg3, arg4);
        //}

        //if self.current_pc == 0xc0 {
        //    let function = self.regs[9];
        //    let arg1 = self.regs[4];
        //    let arg2 = self.regs[5];
        //
        //    self.disassemble_bios_call_c0(function, arg1, arg2);
        //}

        self.execute(op);
    }

    fn update_irq(&mut self) {
        if self.bus.check_interrupts() {
            self.cop0.set_interrupt_bit();
        } else {
            self.cop0.clear_interrupt_bit();
        }
    }

    fn disassemble_bios_call_a0(&mut self, function: u32, arg1: u32, arg2: u32, arg3: u32) {
        match function {
            0x13 => println!("[BIOS] [INFO] SaveState({:#x})", arg1),
            0x15 => println!("[BIOS] [INFO] strcat(\"{}\", \"{}\")", self.disassemble_bios_string(arg1), self.disassemble_bios_string(arg2)),
            0x16 => println!("[BIOS] [INFO] strncat(\"{}\", \"{}\", {:#x})", self.disassemble_bios_string(arg1), self.disassemble_bios_string(arg2), arg3),
            0x17 => println!("[BIOS] [INFO] strcmp(\"{}\", \"{}\")", self.disassemble_bios_string(arg1), self.disassemble_bios_string(arg2)),
            0x18 => println!("[BIOS] [INFO] strncmp(\"{}\", \"{}\", {:#x})", self.disassemble_bios_string(arg1), self.disassemble_bios_string(arg2), arg3),
            0x19 => println!("[BIOS] [INFO] strcpy({:#x}, \"{}\")", arg1, self.disassemble_bios_string(arg2)),
            0x1a => println!("[BIOS] [INFO] strncpy({:#x}, \"{}\", {:#x})", arg1, self.disassemble_bios_string(arg2), arg3),
            0x1b => println!("[BIOS] [INFO] strlen(\"{}\")", self.disassemble_bios_string(arg1)),
            0x25 => println!("[BIOS] [INFO] toupper('{}')", arg1 as u8 as char),
            0x26 => println!("[BIOS] [INFO] tolower('{}')", arg1 as u8 as char),
            0x27 => println!("[BIOS] [INFO] bcopy({:#x}, {:#x}, {:#x})", arg1, arg2, arg3),
            0x28 => println!("[BIOS] [INFO] bzero({:#x}, {:#x})", arg1, arg2),
            0x2a => println!("[BIOS] [INFO] memcpy({:#x}, {:#x}, {:#x})", arg1, arg2, arg3),
            0x2b => println!("[BIOS] [INFO] memset({:#x}, {:#x}, {:#x})", arg1, arg2, arg3),
            0x33 => println!("[BIOS] [INFO] malloc({:#x})", arg1),
            0x34 => println!("[BIOS] [INFO] free({:#x})", arg1),
            0x39 => println!("[BIOS] [INFO] InitHeap({:#x}, {:#x})", arg1, arg2),
            0x3f => (), //println!("[BIOS] [INFO] printf({})", self.disassemble_bios_string(arg1)),
            0x40 => println!("[BIOS] [INFO] SystemErrorUnresolvedException()"),
            0x44 => println!("[BIOS] [INFO] FlushCache()"),
            0x49 => println!("[BIOS] [INFO] GPU_cw({:#x})", arg1),
            0x70 => println!("[BIOS] [INFO] _bu_init()"),
            0x71 => println!("[BIOS] [INFO] CdInit()"),
            0x72 => println!("[BIOS] [INFO] CdRemove()"),
            0x78 => {
                let (mm, ss, ff) = self.disassemble_timecode(arg1);
                println!("[BIOS] [INFO] CdAsyncSeekL({}, {}, {})", mm, ss, ff);
            },
            0x7c => println!("[BIOS] [INFO] CdAsyncGetStatus({:#x})", arg1),
            0x7e => println!("[BIOS] [INFO] CdAsyncReadSector({:#x}, {:#x}, {:#x})", arg1, arg2, arg3),
            0x81 => println!("[BIOS] [INFO] CdAsyncSetMode({:#x})", arg1),
            0x95 => println!("[BIOS] [INFO] CdInitSubFunc()"),
            0x96 => println!("[BIOS] [INFO] AddCDROMDevice()"),
            0x97 => println!("[BIOS] [INFO] AddMemCardDevice()"),
            0x98 => println!("[BIOS] [INFO] AddDuartTtyDevice()"),
            0x99 => println!("[BIOS] [INFO] AddDummyTtyDevice()"),
            0x9f => println!("[BIOS] [INFO] SetMemSize({:#x})", arg1),
            0xa1 => println!("[BIOS] [INFO] SystemErrorBootOrDiskFailure('{}', {:#x})", arg1 as u8 as char, arg2),
            0xa2 => println!("[BIOS] [INFO] EnqueueCdIntr()"),
            0xa3 => println!("[BIOS] [INFO] DequeueCdIntr()"),
            0xa7 => println!("[BIOS] [INFO] bu_callback_okay()"),
            0xa8 => println!("[BIOS] [INFO] bu_callback_err_write()"),
            0xa9 => println!("[BIOS] [INFO] bu_callback_err_busy()"),
            0xaa => println!("[BIOS] [INFO] bu_callback_err_eject()"),
            0xab => println!("[BIOS] [INFO] _card_info({})", arg1),
            0xac => println!("[BIOS] [INFO] _card_async_load_directory({})", arg1),
            0xad => println!("[BIOS] [INFO] set_card_auto_format({:#x})", arg1),
            0xae => println!("[BIOS] [INFO] bu_callback_err_prev_write()"),
            _ => panic!("[BIOS] [ERROR] Unrecognised a0 bios call: 0x{:02x}", function),
        };
    }

    fn disassemble_bios_call_b0(&mut self, function: u32, arg1: u32, arg2: u32, arg3: u32, arg4: u32) {
        match function {
            0x00 => println!("[BIOS] [INFO] alloc_kernel_memory({:#x})", arg1),
            0x01 => println!("[BIOS] [INFO] free_kernel_memory({:#x})", arg1),
            0x07 => println!("[BIOS] [INFO] DeliverEvent({:#x}, {:#x})", arg1, arg2),
            0x08 => println!("[BIOS] [INFO] OpenEvent({:#x}, {:#x}, {:#x}, {:#x})", arg1, arg2, arg3, arg4),
            0x09 => println!("[BIOS] [INFO] CloseEvent({:#x})", arg1),
            0x0a => println!("[BIOS] [INFO] WaitEvent({:#x})", arg1),
            0x0b => (), //println!("[BIOS] [INFO] TestEvent({:#x})", arg1),
            0x0c => println!("[BIOS] [INFO] EnableEvent({:#x})", arg1),
            0x0d => println!("[BIOS] [INFO] DisableEvent({:#x})", arg1),
            0x0e => println!("[BIOS] [INFO] OpenThread({:#x}, {:#x}, {:#x})", arg1, arg2, arg3),
            0x0f => println!("[BIOS] [INFO] CloseThread({:#x})", arg1),
            0x10 => println!("[BIOS] [INFO] ChangeThread({:#x})", arg1),
            0x12 => println!("[BIOS] [INFO] InitPad({:#x}, {:#x}, {:#x}, {:#x})", arg1, arg2, arg3, arg4),
            0x13 => println!("[BIOS] [INFO] StartPad()"),
            0x14 => println!("[BIOS] [INFO] StopPad()"),
            0x15 => println!("[BIOS] [INFO] OutdatedPadInitAndStart({:#x}, {:#x}, {:#x}, {:#x})", arg1, arg2, arg3, arg4),
            0x16 => println!("[BIOS] [INFO] OutdatedPadGetButtons()"),
            0x17 => (), //println!("[BIOS] [INFO] ReturnFromException()"),
            0x18 => println!("[BIOS] [INFO] SetDefaultExitFromException()"),
            0x19 => println!("[BIOS] [INFO] SetCustomExitFromException({:#x})", arg1),
            0x20 => println!("[BIOS] [INFO] UnDeliverEvent({:#x}, {:#x})", arg1, arg2),
            0x32 => println!("[BIOS] [INFO] FileOpen({}, {:#x})", self.disassemble_bios_string(arg1), arg2),
            0x33 => println!("[BIOS] [INFO] FileSeek({:#x}, {:#x}, {})", arg1, arg2, arg3),
            0x34 => println!("[BIOS] [INFO] FileRead({:#x}, {:#x}, {:#x})", arg1, arg2, arg3),
            0x35 => println!("[BIOS] [INFO] FileWrite({:#x}, {:#x}, {:#x})", arg1, arg2, arg3),
            0x36 => println!("[BIOS] [INFO] FileClose({:#x})", arg1),
            0x3c => println!("[BIOS] [INFO] std_in_getchar()"),
            0x3d => (), //println!("[BIOS] [INFO] std_out_putchar('{}')", arg1 as u8 as char),
            0x3f => println!("[BIOS] [INFO] std_out_puts(\"{}\")", self.disassemble_bios_string(arg1)),
            0x47 => println!("[BIOS] [INFO] AddDevice({:#x})", arg1),
            0x4a => println!("[BIOS] [INFO] InitCard({:#x})", arg1),
            0x4b => println!("[BIOS] [INFO] StartCard()"),
            0x4c => println!("[BIOS] [INFO] StopCard()"),
            0x4d => println!("[BIOS] [INFO] _card_info_subfunc({})", arg1),
            0x4e => println!("[BIOS] [INFO] write_card_sector({}, {:#x}, {:#x})", arg1, arg2, arg3),
            0x4f => println!("[BIOS] [INFO] read_card_sector({}, {:#x}, {:#x})", arg1, arg2, arg3),
            0x50 => println!("[BIOS] [INFO] allow_new_card()"),
            0x56 => println!("[BIOS] [INFO] GetC0Table()"),
            0x57 => println!("[BIOS] [INFO] GetB0Table()"),
            0x58 => println!("[BIOS] [INFO] get_bu_callback_port()"),
            0x5b => println!("[BIOS] [INFO] ChangeClearPad({})", arg1),
            _ => panic!("[BIOS] [ERROR] Unrecognised b0 bios call: 0x{:02x}", function),
        };
    }

    fn disassemble_bios_call_c0(&mut self, function: u32, arg1: u32, arg2: u32) {
        match function {
            0x00 => println!("[BIOS] [INFO] EnqueueTimerAndVblankIrqs({})", arg1),
            0x01 => println!("[BIOS] [INFO] EnqueueSyscallHandler({})", arg1),
            0x02 => println!("[BIOS] [INFO] SysEnqIntRP({}, {:#x})", arg1, arg2),
            0x03 => println!("[BIOS] [INFO] SysDeqIntRP({}, {:#x})", arg1, arg2),
            0x07 => println!("[BIOS] [INFO] InstallExceptionHandlers()"),
            0x08 => println!("[BIOS] [INFO] SysInitMemory({:#x}, {:#x})", arg1, arg2),
            0x0a => println!("[BIOS] [INFO] ChangeClearRCnt({}, {})", arg1, arg2),
            0x0c => println!("[BIOS] [INFO] InitDefInt({})", arg1),
            0x12 => println!("[BIOS] [INFO] InstallDevices({})", arg1),
            0x1c => println!("[BIOS] [INFO] AdjustA0Table()"),
            _ => panic!("[BIOS] [ERROR] Unrecognised c0 bios call: 0x{:02x}", function),
        };
    }

    fn disassemble_bios_string(&mut self, address: u32) -> String {
        let mut string = String::new();
        let mut length = 0;

        let mut character = self.load8(address) as char;
        while character != '\0' {
            string.push(character);
            length += 1;

            character = self.load8(address + length) as char;
        }

        string.pop();
        string
    }

    fn disassemble_timecode(&mut self, timecode: u32) -> (u8, u8, u8) {
        let mm = (timecode >> 16) as u8;
        let ss = (timecode >> 8) as u8;
        let ff = timecode as u8;

        (util::bcd_to_u8(mm), util::bcd_to_u8(ss), util::bcd_to_u8(ff))
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
            Mult(rs, rt) => self.op_mult(rs, rt),
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
            Bcond(rs, rt, offset) => self.op_bcond(rs, rt, offset),
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
            Mfc2(rd, rt) => self.op_mfc2(rd, rt),
            Cfc2(rd, rt) => self.op_cfc2(rd, rt),
            Mtc2(rd, rt) => self.op_mtc2(rd, rt),
            Ctc2(rd, rt) => self.op_ctc2(rd, rt),
            Bc2f => panic!("BC2F"),
            Bc2t => panic!("BC2T"),
            Cop2(function) => self.op_cop2(function),
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
            Lwc2(rt, rs, offset) => self.op_lwc2(rt, rs, offset),
            Swc2(rt, rs, offset) => self.op_swc2(rt, rs, offset),
            Unknown(instruction) => self.op_illegal(),//panic!("[CPU] [ERROR] 0x{:08x}: Unknown instruction 0x{:08x} (0x{:02x}:{:02x})", self.current_pc, instruction, instruction >> 26, instruction & 0x3f),
        }
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
        self.branch = true;
        self.new_pc = self.reg(rs);
        
        self.execute_load_delay();
    }

    fn op_jalr(&mut self, rd: usize, rs: usize) {
        let ra = self.new_pc;

        self.branch = true;
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

    fn op_and(&mut self, rd: usize, rs: usize, rt: usize){
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

        if result {
            self.branch(offset);
        }
    }

    fn op_j(&mut self, target: u32) {
        self.execute_load_delay();

        self.branch = true;
        self.new_pc = (self.pc & 0xf000_0000) | (target << 2);
    }

    fn op_jal(&mut self, target: u32) {
        self.execute_load_delay();

        let ra = self.new_pc;

        self.branch = true;
        self.new_pc = (self.pc & 0xf000_0000) | (target << 2);

        self.set_reg(31, ra);
    }

    fn op_beq(&mut self, rs: usize, rt: usize, offset: u32) {
        if self.reg(rs) == self.reg(rt) {
            self.branch(offset);
        }

        self.execute_load_delay();
    }

    fn op_bne(&mut self, rs: usize, rt: usize, offset: u32) {
        if self.reg(rs) != self.reg(rt) {
            self.branch(offset);
        }

        self.execute_load_delay();
    }

    fn op_blez(&mut self, rs: usize, offset: u32) {
        if self.reg(rs) as i32 <= 0 {
            self.branch(offset);
        }

        self.execute_load_delay();
    }

    fn op_bgtz(&mut self, rs: usize, offset: u32) {
        if self.reg(rs) as i32 > 0 {
            self.branch(offset);
        }

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

    fn op_mfc0(&mut self, rd: usize, rt: usize) {
        if rd == 0 || rd == 2 || rd == 4 || rd == 10 || rd >= 32 {
            self.enter_exception(Exception::Reserved);
            return;
        }

        let v = self.cop0.read(rd);

        self.execute_load_delay();

        self.ld_slot = (rt, v);
    }

    fn op_mtc0(&mut self, rd: usize, rt: usize) {
        let v = self.reg(rt);
        self.cop0.write(rd, v);

        self.execute_load_delay();
    }

    fn op_rfe(&mut self) {
        self.execute_load_delay();

        self.cop0.leave_exception();
    }
    
    fn op_mfc2(&mut self, rd: usize, rt: usize) {
        let v = self.gte.read_data(rd);

        self.execute_load_delay();

        self.ld_slot = (rt, v);
    }

    fn op_cfc2(&mut self, rd: usize, rt: usize) {
        let v = self.gte.read_control(rd);

        self.execute_load_delay();

        self.ld_slot = (rt, v);
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

    fn op_cop2(&mut self, function: u32) {
        self.gte.execute(function);

        self.execute_load_delay();
    }

    fn op_lb(&mut self, rt: usize, rs: usize, offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);
        let v = self.load8(addr) as i8 as u32;

        self.update_load_delay(rt, v);
    }

    fn op_lh(&mut self, rt: usize, rs: usize, offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);
        let v = self.load16(addr) as i16 as u32;

        if addr & 0x01 != 0 {
            self.cop0.set_bad_vaddr(self.pc);
            self.enter_exception(Exception::AddrLoad);
            return;
        }

        self.update_load_delay(rt, v);
    }

    fn op_lwl(&mut self, rt: usize, rs: usize, offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);

        let mut current = self.reg(rt);
        let aligned = self.load32(addr & 0xffff_fffc);

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
            3 => (current & 0x0000_0000) | (aligned << 0),
            _ => unreachable!(),
        };
    }

    fn op_lw(&mut self, rt: usize, rs: usize, offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);
        let v = self.load32(addr);

        if addr & 0x03 != 0 {
            self.cop0.set_bad_vaddr(self.pc);
            self.enter_exception(Exception::AddrLoad);
            return;
        }

        self.update_load_delay(rt, v);
    }

    fn op_lbu(&mut self, rt: usize, rs: usize, offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);
        let v = self.load8(addr) as u32;

        self.update_load_delay(rt, v);
    }

    fn op_lhu(&mut self, rt: usize, rs: usize, offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);
        let v = self.load16(addr) as u32;

        if addr & 0x01 != 0 {
            self.cop0.set_bad_vaddr(self.pc);
            self.enter_exception(Exception::AddrLoad);
            return;
        }

        self.update_load_delay(rt, v);
    }

    fn op_lwr(&mut self, rt: usize, rs: usize, offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);

        let mut current = self.reg(rt);
        let aligned = self.load32(addr & 0xffff_fffc);

        if self.ld_slot.0 == rt {
            current = self.ld_slot.1;
        } else {
            self.execute_load_delay();
        }

        self.ld_slot.0 = rt;

        self.ld_slot.1 = match addr & 0x03 {
            0 => (current & 0x0000_0000) | (aligned >> 0),
            1 => (current & 0xff00_0000) | (aligned >> 8),
            2 => (current & 0xffff_0000) | (aligned >> 16),
            3 => (current & 0xffff_ff00) | (aligned >> 24),
            _ => unreachable!(),
        };
    }

    fn op_sb(&mut self, rt: usize, rs: usize, offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);
        let v = self.reg(rt);

        self.store8(addr, v);

        self.execute_load_delay();
    }

    fn op_sh(&mut self, rt: usize, rs: usize, offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);
        let v = self.reg(rt);

        self.execute_load_delay();

        if addr & 0x01 != 0 {
            self.enter_exception(Exception::AddrStore);
            return;
        }

        self.store16(addr, v);
    }

    fn op_swl(&mut self, rt: usize, rs: usize, offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);
        let value = self.reg(rt);

        let current = self.load32(addr & 0xffff_fffc);

        let v = match addr & 0x03 {
            0 => (current & 0xffff_ff00) | (value >> 24),
            1 => (current & 0xffff_0000) | (value >> 16),
            2 => (current & 0xff00_0000) | (value >> 8),
            3 => (current & 0x0000_0000) | (value >> 0),
            _ => unreachable!(),
        };

        self.store32(addr & 0xffff_fffc, v);

        self.execute_load_delay();
    }

    fn op_sw(&mut self, rt: usize, rs: usize, offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);
        let v = self.reg(rt);

        self.execute_load_delay();

        if addr & 0x03 != 0 {
            self.enter_exception(Exception::AddrStore);
            return;
        }

        self.store32(addr, v);
    }

    fn op_swr(&mut self, rt: usize, rs: usize, offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);
        let value = self.reg(rt);

        let current = self.load32(addr & 0xffff_fffc);

        let v = match addr & 0x03 {
            0 => (current & 0x0000_0000) | (value << 0),
            1 => (current & 0x0000_00ff) | (value << 8),
            2 => (current & 0x0000_ffff) | (value << 16),
            3 => (current & 0x00ff_ffff) | (value << 24),
            _ => unreachable!(),
        };

        self.store32(addr & 0xffff_fffc, v);

        self.execute_load_delay();
    }

    fn op_lwc2(&mut self, rt: usize, rs: usize, offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);

        self.execute_load_delay();

        if addr & 0x03 != 0 {
            self.cop0.set_bad_vaddr(self.pc);
            self.enter_exception(Exception::AddrLoad);
            return;
        }

        let v = self.load32(addr);
        self.gte.write_data(rt, v);
    }

    fn op_swc2(&mut self, rt: usize, rs: usize, offset: u32) {
        let addr = self.reg(rs).wrapping_add(offset);
        let v = self.gte.read_data(rt);

        self.execute_load_delay();

        if addr & 0x03 != 0 {
            self.enter_exception(Exception::AddrStore);
            return;
        }

        self.store32(addr, v);
    }

    fn op_illegal(&mut self) {
        self.execute_load_delay();

        self.enter_exception(Exception::Reserved);
    }

    fn reg(&self, index: usize) -> u32 {
        self.regs[index]
    }

    fn set_reg(&mut self, index: usize, value: u32) {
        self.regs[index] = value;
        self.regs[0] = 0;
    }

    fn branch(&mut self, offset: u32) {
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

    fn update_load_delay(&mut self, index: usize, value: u32) {
        if self.ld_slot.0 == index {
            self.ld_slot.0 = 0;
        }
        
        self.execute_load_delay();

        self.ld_slot = (index, value);
    }

    fn fetch32(&mut self) -> u32 {
        let pc = self.pc;
        let physical_address = self.translate_address(pc);

        self.bus.load(physical_address, false)
    }

    fn translate_address(&self, virtual_address: u32) -> u32 {
        let virtual_region = VirtualRegion::from_u32(virtual_address);
        virtual_region.translate_address(virtual_address)
    }

    fn load(&mut self, address: u32, half: bool) -> u32 {
        self.last_load = address;

        let physical_address = self.translate_address(address);

        if !self.cop0.isolate_cache() {
            self.bus.load(physical_address, half)
        } else {
            0
        }
    }

    fn load8(&mut self, address: u32) -> u8 {
        self.load(address, false) as u8
    }

    fn load16(&mut self, address: u32) -> u16 {
        self.load(address, true) as u16
    }

    fn load32(&mut self, address: u32) -> u32 {
        self.load(address, false)
    }

    fn store(&mut self, width: BusWidth, address: u32, value: u32) {
        self.last_store = address;

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

    pub fn debug_current_pc(&self) -> u32 {
        self.current_pc
    }

    pub fn debug_last_load(&self) -> u32 {
        self.last_load
    }

    pub fn debug_last_store(&self) -> u32 {
        self.last_store
    }

    pub fn debug_register(&self, index: usize) -> u32 {
        self.regs[index]
    }

    pub fn debug_load(&self, address: u32) -> Result<u32, ()> {
        let physical_address = self.translate_address(address);

        self.bus.debug_load(BusWidth::WORD, physical_address)
    }
}