use std::fmt::Write as FmtWrite;
use std::io::{stdin, stdout, Write};

use super::psx::System;
use super::psx::cpu::{R3000A, REGISTERS};
use super::psx::cpu::ops::Operation;
use super::psx::interrupt::Interrupt;

#[derive(Clone)]
pub struct Breakpoint {
    address: u32,
}

impl Breakpoint {
    pub fn new(address: u32) -> Breakpoint {
        Breakpoint {
            address: address,
        }
    }
}

pub enum Command {
    Br(String),
    Bw(String),
    Bx(String),
    Continue,
    Step,
    Trace,
    Quit,
    Unknown,
}

impl Command {
    fn from_stdin() -> Command {
        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();
        input = input.to_ascii_lowercase();

        let vec: Vec<&str> = input.trim().split(" ").collect();
        vec.into()
    }
}

impl<'a> From<Vec<&'a str>> for Command {
    fn from(command: Vec<&str>) -> Command {
        use self::Command::*;

        match command.as_slice() {
            ["br", address] => Br(address.to_string()),
            ["bw", address] => Bw(address.to_string()),
            ["bx", address] => Bx(address.to_string()),
            ["continue"] => Continue,
            ["step"] => Step,
            ["trace"] => Trace,
            ["quit"] => Quit,
            _ => Unknown,
        }
    }
}

pub struct Debugger {
    system: System,
    read_breakpoints: Vec<Breakpoint>,
    write_breakpoints: Vec<Breakpoint>,
    execute_breakpoints: Vec<Breakpoint>,

    stdout_buffer: String,

    running: bool,
    cont: bool,
    step: bool,
    trace: bool,
}

impl Debugger {
    pub fn new(system: System) -> Debugger
    {
        Debugger {
            system: system,
            read_breakpoints: Vec::new(),
            write_breakpoints: Vec::new(),
            execute_breakpoints: Vec::new(),

            stdout_buffer: String::new(),

            running: true,
            cont: false,
            step: false,
            trace: false,
        }
    }

	pub fn reset(&mut self)
	{
		self.system.reset();
	}

    pub fn run(&mut self)
    {
        while self.running {
            for _ in 0..285620 {
                if !self.cont {
                    print!("rpsx> ");
                    stdout().flush().unwrap();

                    self.process_command(Command::from_stdin());
                }

                if self.cont || self.step {
                    self.system.run();

                    for _ in 0..2 {
                        self.system.tick();
                    }

                    if self.trace {
                        let cpu = self.system.cpu();
                        let inst_str = Debugger::disassemble_current_instruction(cpu);

                        println!("{}", inst_str);
                    }

                    self.process_breakpoints();
                    self.process_stdout();

                    self.step = false;
                }
            }

            self.system.render_frame();
            self.system.set_interrupt(Interrupt::Vblank);
        }
    }

    fn process_command(&mut self, command: Command) {
        use self::Command::*;

        match command {
            Br(address) => {
                let address_option = u32::from_str_radix(&address.as_str(), 16);

                if !address_option.is_ok() {
                    println!("[DEBUG] [ERROR] Invalid address entered");
                    return;
                }

                let a = address_option.unwrap();

                for bp in self.read_breakpoints.clone() {
                    if bp.address == a {
                        println!("[DEBUG] [ERROR] Breakpoint already defined");
                        return;
                    }
                }

                self.read_breakpoints.push(Breakpoint::new(a));
                println!("[DEBUG] [INFO] Breakpoint set");
            },
            Bw(address) => {
                let address_option = u32::from_str_radix(&address.as_str(), 16);

                if !address_option.is_ok() {
                    println!("[DEBUG] [ERROR] Invalid address entered");
                    return;
                }

                let a = address_option.unwrap();

                for bp in self.write_breakpoints.clone() {
                    if bp.address == a {
                        println!("[DEBUG] [ERROR] Breakpoint already defined");
                        return;
                    }
                }

                self.write_breakpoints.push(Breakpoint::new(a));
                println!("[DEBUG] [INFO] Breakpoint set");
            },
            Bx(address) => {
                let address_option = u32::from_str_radix(&address.as_str(), 16);

                if !address_option.is_ok() {
                    println!("[DEBUG] [ERROR] Invalid address entered");
                    return;
                }

                let a = address_option.unwrap();

                for bp in self.execute_breakpoints.clone() {
                    if bp.address == a {
                        println!("[DEBUG] [ERROR] Breakpoint already defined");
                        return;
                    }
                }

                self.execute_breakpoints.push(Breakpoint::new(a));
                println!("[DEBUG] [INFO] Breakpoint set");
            },
            Continue => self.cont = true,
            Step => self.step = true,
            Trace => {
                self.trace = !self.trace;

                let state = match self.trace {
                    true => "enabled",
                    false => "disabled",
                };

                println!("[DEBUG] [INFO] Trace {}", state);
            },
            Quit => self.running = false,
            Unknown => println!("[DEBUG] [ERROR] Unrecognised command"),
        }
    }

    fn process_breakpoints(&mut self) {
        let cpu = self.system.cpu();
        let pc = cpu.debug_current_pc();
        let la = cpu.debug_last_load();
        let sa = cpu.debug_last_store();

        for bp in self.read_breakpoints.clone() {
            if bp.address == la {
                println!("[DEBUG] [INFO] Breakpoint (Read) hit at 0x{:04x}", la);
                self.cont = false;
                return;
            }
        }

        for bp in self.write_breakpoints.clone() {
            if bp.address == sa {
                println!("[DEBUG] [INFO] Breakpoint (Write) hit at 0x{:04x}", sa);
                self.cont = false;
                return;
            }
        }

        for bp in self.execute_breakpoints.clone() {
            if bp.address == pc {
                println!("[DEBUG] [INFO] Breakpoint (Execute) hit at 0x{:04x}", pc);
                self.cont = false;
                return;
            }
        }
    }

    fn process_stdout(&mut self) {
        let cpu = self.system.cpu();
        let pc = cpu.debug_current_pc();

        if pc == 0xb0 {
            let function = cpu.debug_register(9);

            if function == 0x3d {
                let arg = cpu.debug_register(4) as u8 as char;

                if arg == '\n' {
                    if self.stdout_buffer.len() > 0 {
                        println!("[DEBUG] [INFO] stdout: {}", self.stdout_buffer);
                        self.stdout_buffer.clear();
                    }
                } else {
                    self.stdout_buffer.push(arg);
                }
            }
        }
    }

    fn disassemble_current_instruction(cpu: &R3000A) -> String {
        let current_pc = cpu.debug_current_pc();

        Debugger::disassemble_instruction(cpu, current_pc)
    }

    fn disassemble_instruction(cpu: &R3000A, address: u32) -> String {
        use self::Operation::*;

        let inst_bytes = cpu.debug_load(address).unwrap();
        let mut inst_str = String::new();

        write!(&mut inst_str, "0x{:08x}: ", address).unwrap();

        let current_pc = cpu.debug_current_pc();
        let current_operation: Operation = inst_bytes.into();
        
        match current_operation {
            Sll(rd, rt, shift) => write!(&mut inst_str, "SLL {}, {}, {}", REGISTERS[rd], REGISTERS[rt], shift),
            Srl(rd, rt, shift) => write!(&mut inst_str, "SRL {}, {}, {}", REGISTERS[rd], REGISTERS[rt], shift),
            Sra(rd, rt, shift) => write!(&mut inst_str, "SRA {}, {}, {}", REGISTERS[rd], REGISTERS[rt], shift),
            Sllv(rd, rt, rs) => write!(&mut inst_str, "SLLV {}, {}, {}", REGISTERS[rd], REGISTERS[rt], REGISTERS[rs]),
            Srlv(rd, rt, rs) => write!(&mut inst_str, "SRLV {}, {}, {}", REGISTERS[rd], REGISTERS[rt], REGISTERS[rs]),
            Srav(rd, rt, rs) => write!(&mut inst_str, "SRLV {}, {}, {}", REGISTERS[rd], REGISTERS[rt], REGISTERS[rs]),
            Jr(rs) => write!(&mut inst_str, "JR {}", REGISTERS[rs]),
            Jalr(rd, rs) => write!(&mut inst_str, "JR {}, {}", REGISTERS[rd], REGISTERS[rs]),
            Syscall => write!(&mut inst_str, "SYSCALL"),
            Break => write!(&mut inst_str, "BREAK"),
            Mfhi(rd) => write!(&mut inst_str, "MFHI {}", REGISTERS[rd]),
            Mthi(rs) => write!(&mut inst_str, "MTHI {}", REGISTERS[rs]),
            Mflo(rd) => write!(&mut inst_str, "MFLO {}", REGISTERS[rd]),
            Mtlo(rs) => write!(&mut inst_str, "MTLO {}", REGISTERS[rs]),
            Multu(rs, rt) => write!(&mut inst_str, "MULTU {}, {}", REGISTERS[rs], REGISTERS[rt]),
            Div(rs, rt) => write!(&mut inst_str, "DIV {}, {}", REGISTERS[rs], REGISTERS[rt]),
            Divu(rs, rt) => write!(&mut inst_str, "DIVU {}, {}", REGISTERS[rs], REGISTERS[rt]),
            Add(rd, rs, rt) => write!(&mut inst_str, "ADD {}, {}, {}", REGISTERS[rd], REGISTERS[rs], REGISTERS[rt]),
            Addu(rd, rs, rt) => write!(&mut inst_str, "ADDU {}, {}, {}", REGISTERS[rd], REGISTERS[rs], REGISTERS[rt]),
            Sub(rd, rs, rt) => write!(&mut inst_str, "SUB {}, {}, {}", REGISTERS[rd], REGISTERS[rs], REGISTERS[rt]),
            Subu(rd, rs, rt) => write!(&mut inst_str, "SUBU {}, {}, {}", REGISTERS[rd], REGISTERS[rs], REGISTERS[rt]),
            And(rd, rs, rt) => write!(&mut inst_str, "AND {}, {}, {}", REGISTERS[rd], REGISTERS[rs], REGISTERS[rt]),
            Or(rd, rs, rt) => write!(&mut inst_str, "OR {}, {}, {}", REGISTERS[rd], REGISTERS[rs], REGISTERS[rt]),
            Xor(rd, rs, rt) => write!(&mut inst_str, "OR {}, {}, {}", REGISTERS[rd], REGISTERS[rs], REGISTERS[rt]),
            Nor(rd, rs, rt) => write!(&mut inst_str, "OR {}, {}, {}", REGISTERS[rd], REGISTERS[rs], REGISTERS[rt]),
            Slt(rd, rs, rt) => write!(&mut inst_str, "SLT {}, {}, {}", REGISTERS[rd], REGISTERS[rs], REGISTERS[rt]),
            Sltu(rd, rs, rt) => write!(&mut inst_str, "SLTU {}, {}, {}", REGISTERS[rd], REGISTERS[rs], REGISTERS[rt]),
            Bltz(rs, offset) => write!(&mut inst_str, "BLTZ {}, 0x{:04x}", REGISTERS[rs], offset),
            Bgez(rs, offset) => write!(&mut inst_str, "BGEZ {}, 0x{:04x}", REGISTERS[rs], offset),
            Bltzal(rs, offset) => write!(&mut inst_str, "BLTZAL {}, 0x{:04x}", REGISTERS[rs], offset),
            Bgezal(rs, offset) => write!(&mut inst_str, "BGEZAL {}, 0x{:04x}", REGISTERS[rs], offset),
            J(target) => write!(&mut inst_str, "J 0x{:04x}", (current_pc & 0xf000_0000) | (target << 2)),
            Jal(target) => write!(&mut inst_str, "JAL 0x{:04x}", (current_pc & 0xf000_0000) | (target << 2)),
            Beq(rs, rt, offset) => write!(&mut inst_str, "BEQ {}, {}, 0x{:04x}", REGISTERS[rs], REGISTERS[rt], offset),
            Bne(rs, rt, offset) => write!(&mut inst_str, "BNE {}, {}, 0x{:04x}", REGISTERS[rs], REGISTERS[rt], offset),
            Blez(rs, offset) => write!(&mut inst_str, "BLEZ {}, 0x{:04x}", REGISTERS[rs], offset),
            Bgtz(rs, offset) => write!(&mut inst_str, "BGTZ {}, 0x{:04x}", REGISTERS[rs], offset),
            Addi(rt, rs, imm) => write!(&mut inst_str, "ADDI {}, {}, 0x{:04x}", REGISTERS[rt], REGISTERS[rs], imm),
            Addiu(rt, rs, imm) => write!(&mut inst_str, "ADDIU {}, {}, 0x{:04x}", REGISTERS[rt], REGISTERS[rs], imm),
            Slti(rt, rs, imm) => write!(&mut inst_str, "SLTI {}, {}, 0x{:04x}", REGISTERS[rt], REGISTERS[rs], imm),
            Sltiu(rt, rs, imm) => write!(&mut inst_str, "SLTIU {}, {}, 0x{:04x}", REGISTERS[rt], REGISTERS[rs], imm),
            Andi(rt, rs, imm) => write!(&mut inst_str, "ANDI {}, {}, 0x{:04x}", REGISTERS[rt], REGISTERS[rs], imm),
            Ori(rt, rs, imm) => write!(&mut inst_str, "ORI {}, {}, 0x{:04x}", REGISTERS[rt], REGISTERS[rs], imm),
            Xori(rt, rs, imm) => write!(&mut inst_str, "XORI {}, {}, 0x{:04x}", REGISTERS[rt], REGISTERS[rs], imm),
            Lui(rt, imm) => write!(&mut inst_str, "LUI {}, 0x{:04x}", REGISTERS[rt], imm),
            Mfc0(rd, rt) =>  write!(&mut inst_str, "MFC0 {}, cop0r{}", REGISTERS[rt], rd),
            Mtc0(rd, rt) =>  write!(&mut inst_str, "MTC0 {}, cop0r{}", REGISTERS[rt], rd),
            Rfe => write!(&mut inst_str, "RFE"),
            Lb(rt, rs, offset) => write!(&mut inst_str, "LB {}, 0x{:04x}({})", REGISTERS[rt], offset, REGISTERS[rs]),
            Lh(rt, rs, offset) => write!(&mut inst_str, "LH {}, 0x{:04x}({})", REGISTERS[rt], offset, REGISTERS[rs]),
            Lwl(rt, rs, offset) => write!(&mut inst_str, "LWL {}, 0x{:04x}({})", REGISTERS[rt], offset, REGISTERS[rs]),
            Lw(rt, rs, offset) => write!(&mut inst_str, "LW {}, 0x{:04x}({})", REGISTERS[rt], offset, REGISTERS[rs]),
            Lbu(rt, rs, offset) => write!(&mut inst_str, "LBU {}, 0x{:04x}({})", REGISTERS[rt], offset, REGISTERS[rs]),
            Lhu(rt, rs, offset) => write!(&mut inst_str, "LHU {}, 0x{:04x}({})", REGISTERS[rt], offset, REGISTERS[rs]),
            Lwr(rt, rs, offset) => write!(&mut inst_str, "LWR {}, 0x{:04x}({})", REGISTERS[rt], offset, REGISTERS[rs]),
            Sb(rt, rs, offset) => write!(&mut inst_str, "SB {}, 0x{:04x}({})", REGISTERS[rt], offset, REGISTERS[rs]),
            Sh(rt, rs, offset) => write!(&mut inst_str, "SH {}, 0x{:04x}({})", REGISTERS[rt], offset, REGISTERS[rs]),
            Swl(rt, rs, offset) => write!(&mut inst_str, "SWL {}, 0x{:04x}({})", REGISTERS[rt], offset, REGISTERS[rs]),
            Sw(rt, rs, offset) => write!(&mut inst_str, "SW {}, 0x{:04x}({})", REGISTERS[rt], offset, REGISTERS[rs]),
            Swr(rt, rs, offset) => write!(&mut inst_str, "SWR {}, 0x{:04x}({})", REGISTERS[rt], offset, REGISTERS[rs]),
            Unknown(opcode) => write!(&mut inst_str, "UNKNOWN 0x{:08x}", opcode),
        }.unwrap();

        inst_str
    }
}