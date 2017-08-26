use super::{ CPU, Instruction };
use cpu::cpu::Exception;

pub fn op_sll(cpu: &mut CPU, instruction: Instruction) {
	let rd  = instruction.rd();
	let rt  = instruction.rt();
	let shift = instruction.shift();

	//println!("SLL ${}, ${}, {}", rd, rt, shift);

	let v = cpu.reg(rt) << shift;
	cpu.set_reg(rd, v);
}

pub fn op_srl(cpu: &mut CPU, instruction: Instruction) {
	let rd    = instruction.rd();
	let rt    = instruction.rt();
	let shift = instruction.shift();

	//println!("SRL ${}, ${}, {}", rd, rt, shift);

	let v = cpu.reg(rt) >> shift;
	cpu.set_reg(rd, v);
}

pub fn op_sra(cpu: &mut CPU, instruction: Instruction) {
	let rd    = instruction.rd();
	let rt    = instruction.rt();
	let shift = instruction.shift();

	//println!("SRA ${}, ${}, {}", rd, rt, shift);

	let v = (cpu.reg(rt) as i32) >> shift;
	cpu.set_reg(rd, v as u32);
}

pub fn op_sllv(cpu: &mut CPU, instruction: Instruction) {
	let rd = instruction.rd();
	let rt = instruction.rt();
	let rs = instruction.rs();

	//println!("SLL ${}, ${}, ${}", rd, rt, rs);

	let v = cpu.reg(rt) << (cpu.reg(rs) & 0b11111);
	cpu.set_reg(rd, v);
}

pub fn op_srlv(cpu: &mut CPU, instruction: Instruction) {
	let rd = instruction.rd();
	let rt = instruction.rt();
	let rs = instruction.rs();

	//println!("SRLV ${}, ${}, {}", rd, rt, rs);

	let v = cpu.reg(rt) >> (cpu.reg(rs) & 0b11111);
	cpu.set_reg(rd, v);
}

pub fn op_srav(cpu: &mut CPU, instruction: Instruction) {
	let rd = instruction.rd();
	let rt = instruction.rt();
	let rs = instruction.rs();

	//println!("SRAV ${}, ${}, ${}", rd, rt, rs);

	let v = (cpu.reg(rt) as i32) >> (cpu.reg(rs) & 0b11111);
	cpu.set_reg(rd, v as u32);
}

pub fn op_jr(cpu: &mut CPU, instruction: Instruction) {
	let rs = instruction.rs();

	//println!("JR ${}", rs);

	cpu.branch_delay_enabled = true;
	cpu.branch_delay_pc = cpu.reg(rs);
}

pub fn op_jalr(cpu: &mut CPU, instruction: Instruction) {
	let rd = instruction.rd();
	let rs = instruction.rs();
	let pc = cpu.pc;

	//println!("JALR ${}", rs);

	cpu.branch_delay_enabled = true;
	cpu.branch_delay_pc = cpu.reg(rs);
	cpu.set_reg(rd, pc.wrapping_add(8));
}

pub fn op_syscall(cpu: &mut CPU) {
	//println!("SYSCALL");
	cpu.enter_exception(Exception::SYSCALL)
}

pub fn op_break(cpu: &mut CPU) {
	//println!("BREAK");
	cpu.enter_exception(Exception::BREAKPOINT)
}

pub fn op_mfhi(cpu: &mut CPU, instruction: Instruction) {
	let rd = instruction.rd();
	let hi = cpu.hi;

	//println!("MFHI ${}", rd);

	cpu.set_reg(rd, hi);
}

pub fn op_mthi(cpu: &mut CPU, instruction: Instruction) {
	let rs = instruction.rs();

	//println!("MTHI ${}", rs);

	cpu.hi = cpu.reg(rs);
}

pub fn op_mflo(cpu: &mut CPU, instruction: Instruction) {
	let rd = instruction.rd();
	let lo = cpu.lo;

	//println!("MFLO ${}", rd);

	cpu.set_reg(rd, lo);
}

pub fn op_mtlo(cpu: &mut CPU, instruction: Instruction) {
	let rs = instruction.rs();

	//println!("MTLO ${}", rs);

	cpu.lo = cpu.reg(rs);
}

pub fn op_mult(cpu: &mut CPU, instruction: Instruction) {
	let rt = instruction.rt();
	let rs = instruction.rs();

	//println!("MULT ${}, ${}", rs, rt);

	let a = (cpu.reg(rs) as i32) as i64;
	let b = (cpu.reg(rt) as i32) as i64;

	let v = (a * b) as u64;

	cpu.hi = (v >> 32) as u32;
	cpu.lo = v as u32;
}

pub fn op_multu(cpu: &mut CPU, instruction: Instruction) {
	let rt = instruction.rt();
	let rs = instruction.rs();

	//println!("MULTU ${}, ${}", rs, rt);

	let a = cpu.reg(rs) as u64;
	let b = cpu.reg(rt) as u64;

	let v = a * b;

	cpu.hi = (v >> 32) as u32;
	cpu.lo = v as u32;
}

pub fn op_div(cpu: &mut CPU, instruction: Instruction) {
	let rt = instruction.rt();
	let rs = instruction.rs();

	let n = cpu.reg(rs) as i32;
	let d = cpu.reg(rt) as i32;

	//println!("DIV ${}, ${}", rs, rt);

	if d == 0 {
		cpu.hi = n as u32;

		if n >= 0 {
			cpu.lo = 0xffffffff;
		} else {
			cpu.lo = 1;
		}
	} else if n == -0x80000000 && d == -1 {
		cpu.hi = 0;
		cpu.lo = 0x80000000;
	} else {
		cpu.hi = (n % d) as u32;
		cpu.lo = (n / d) as u32;
	}
}

pub fn op_divu(cpu: &mut CPU, instruction: Instruction) {
	let rt = instruction.rt();
	let rs = instruction.rs();

	let n = cpu.reg(rs);
	let d = cpu.reg(rt);

	//println!("DIV ${}, ${}", rs, rt);

	if d == 0 {
		cpu.hi = n;
		cpu.lo = 0xffffffff;
	} else {
		cpu.hi = n % d;
		cpu.lo = n / d;
	}
}

pub fn op_add(cpu: &mut CPU, instruction: Instruction) {
	let rd = instruction.rd();
	let rt = instruction.rt();
	let rs = instruction.rs();

	//println!("ADD ${}, ${}, ${}", rd, rs, rt);

	let v = (cpu.reg(rs) as i32).overflowing_add(cpu.reg(rt) as i32);

	if v.1 {
		cpu.enter_exception(Exception::OVERFLOW)
	}

	cpu.set_reg(rd, v.0 as u32);
}

pub fn op_addu(cpu: &mut CPU, instruction: Instruction) {
	let rd = instruction.rd();
	let rt = instruction.rt();
	let rs = instruction.rs();

	//println!("ADDU ${}, ${}, ${}", rd, rs, rt);

	let v = cpu.reg(rs).wrapping_add(cpu.reg(rt));
	cpu.set_reg(rd, v);
}

pub fn op_sub(cpu: &mut CPU, instruction: Instruction) {
	let rd = instruction.rd();
	let rt = instruction.rt();
	let rs = instruction.rs();

	//println!("SUB ${}, ${}, ${}", rd, rs, rt);

	let v = (cpu.reg(rs) as i32).overflowing_sub(cpu.reg(rt) as i32);

	if v.1 {
		cpu.enter_exception(Exception::OVERFLOW)
	}

	cpu.set_reg(rd, v.0 as u32);
}

pub fn op_subu(cpu: &mut CPU, instruction: Instruction) {
	let rd = instruction.rd();
	let rt = instruction.rt();
	let rs = instruction.rs();

	//println!("SUBU ${}, ${}, ${}", rd, rs, rt);

	let v = cpu.reg(rs).wrapping_sub(cpu.reg(rt));
	cpu.set_reg(rd, v);
}

pub fn op_and(cpu: &mut CPU, instruction: Instruction) {
	let rd = instruction.rd();
	let rt = instruction.rt();
	let rs = instruction.rs();

	//println!("AND ${}, ${}, ${}", rd, rs, rt);

	let v = cpu.reg(rs) & cpu.reg(rt);
	cpu.set_reg(rd, v);	
}

pub fn op_or(cpu: &mut CPU, instruction: Instruction) {
	let rd = instruction.rd();
	let rt = instruction.rt();
	let rs = instruction.rs();

	//println!("OR ${}, ${}, ${}", rd, rs, rt);

	let v = cpu.reg(rs) | cpu.reg(rt);
	cpu.set_reg(rd, v);	
}

pub fn op_xor(cpu: &mut CPU, instruction: Instruction) {
	let rd = instruction.rd();
	let rt = instruction.rt();
	let rs = instruction.rs();

	//println!("XOR ${}, ${}, ${}", rd, rs, rt);

	let v = cpu.reg(rs) ^ cpu.reg(rt);
	cpu.set_reg(rd, v);	
}


pub fn op_nor(cpu: &mut CPU, instruction: Instruction) {
	let rd = instruction.rd();
	let rt = instruction.rt();
	let rs = instruction.rs();

	//println!("NOR ${}, ${}, ${}", rd, rs, rt);

	let v = !(cpu.reg(rs) | cpu.reg(rt));
	cpu.set_reg(rd, v);	
}

pub fn op_slt(cpu: &mut CPU, instruction: Instruction) {
	let rd = instruction.rd();
	let rt = instruction.rt();
	let rs = instruction.rs();

	//println!("SLT ${}, ${}, ${}", rd, rs, rt);

	let v = ((cpu.reg(rs) as i32) < (cpu.reg(rt) as i32)) as u32;
	cpu.set_reg(rd, v);
}

pub fn op_sltu(cpu: &mut CPU, instruction: Instruction) {
	let rd = instruction.rd();
	let rt = instruction.rt();
	let rs = instruction.rs();

	//println!("SLTU ${}, ${}, ${}", rd, rs, rt);	

	let v = (cpu.reg(rs) < cpu.reg(rt)) as u32;
	cpu.set_reg(rd, v);
}

pub fn op_bxx(cpu: &mut CPU, instruction: Instruction) {
	let rs = instruction.rs();
	let offset = instruction.imm_se();
	let pc = cpu.pc;
	let instruction = instruction.as_bytes();

	let bgez = (instruction >> 16) & 0x1;
	let link = (instruction >> 17) & 0xf == 8;

	//let mut op = "BLTZ";
	//if bgez != 0 {
		//op = "BGEZ";
	//}

	//let mut lnk = "";
	if link {
		cpu.set_reg(31, pc.wrapping_add(8));
		//lnk = "AL";
	}

	let v = cpu.reg(rs) as i32;
	let test = (v < 0) as u32 ^ bgez;

	//println!("{}{} ${}, 0x{:04x}", op, lnk, rs, offset);

	if test != 0 {
		cpu.branch(offset);
	}
}

pub fn op_j(cpu: &mut CPU, instruction: Instruction) {
	let target = instruction.target();
	let jump_address = ((cpu.pc + 4) & 0xf000_0000) | (target << 2);

	//println!("J 0x{:08x}", jump_address);

	cpu.branch_delay_enabled = true;
	cpu.branch_delay_pc = jump_address;
}

pub fn op_jal(cpu: &mut CPU, instruction: Instruction) {
	let target = instruction.target();
	let pc = cpu.pc;
	let jump_address = ((pc + 4) & 0xf000_0000) | (target << 2);

	//println!("JAL 0x{:08x}", jump_address);

	cpu.branch_delay_enabled = true;
	cpu.branch_delay_pc = jump_address;
	cpu.set_reg(31, pc.wrapping_add(8));
}

pub fn op_beq(cpu: &mut CPU, instruction: Instruction) {
	let rt = instruction.rt();
	let rs = instruction.rs();
	let offset = instruction.imm_se();

	//println!("BEQ ${}, ${}, 0x{:04x}", rs, rt, offset);

	if cpu.reg(rs) == cpu.reg(rt) {
		cpu.branch(offset);
	}
}

pub fn op_bne(cpu: &mut CPU, instruction: Instruction) {
	let rt = instruction.rt();
	let rs = instruction.rs();
	let offset = instruction.imm_se();

	//println!("BNE ${}, ${}, 0x{:04x}", rs, rt, offset);

	if cpu.reg(rs) != cpu.reg(rt) {
		cpu.branch(offset);
	}
}

pub fn op_blez(cpu: &mut CPU, instruction: Instruction) {
	let rs = instruction.rs();
	let offset = instruction.imm_se();

	//println!("BLEZ ${}, 0x{:04x}", rs, offset);

	if cpu.reg(rs) as i32 <= 0 {
		cpu.branch(offset);
	}
}

pub fn op_bgtz(cpu: &mut CPU, instruction: Instruction) {
	let rs = instruction.rs();
	let offset = instruction.imm_se();

	//println!("BGTZ ${}, 0x{:04x}", rs, offset);

	if cpu.reg(rs) as i32 > 0 {
		cpu.branch(offset);
	}
}

pub fn op_addi(cpu: &mut CPU, instruction: Instruction) {
	let rt  = instruction.rt();
	let rs  = instruction.rs();
	let imm = instruction.imm_se();

	//println!("ADDI ${}, ${}, 0x{:04x}", rt, rs, imm);

	let v = (cpu.reg(rs) as i32).overflowing_add(imm as i32);

	if v.1 {
		cpu.enter_exception(Exception::OVERFLOW)
	}

	cpu.set_reg(rt, v.0 as u32);
}

pub fn op_addiu(cpu: &mut CPU, instruction: Instruction) {
	let rt  = instruction.rt();
	let rs  = instruction.rs();
	let imm = instruction.imm_se();

	//println!("ADDIU ${}, ${}, 0x{:04x}", rt, rs, imm);

	let v = cpu.reg(rs).wrapping_add(imm);
	cpu.set_reg(rt, v);
}

pub fn op_slti(cpu: &mut CPU, instruction: Instruction) {
	let rt = instruction.rt();
	let rs = instruction.rs();
	let imm = instruction.imm_se() as i32;

	//println!("SLTI ${}, ${} 0x{:04x}", rs, rt, imm);

	let v = (cpu.reg(rs) as i32) < imm;
	cpu.set_reg(rt, v as u32);
}

pub fn op_sltiu(cpu: &mut CPU, instruction: Instruction) {
	let rt = instruction.rt();
	let rs = instruction.rs();
	let imm = instruction.imm_se();

	//println!("SLTIU ${}, ${} 0x{:04x}", rs, rt, imm);

	let v = cpu.reg(rs) < imm;
	cpu.set_reg(rt, v as u32);
}

pub fn op_andi(cpu: &mut CPU, instruction: Instruction) {
	let rt  = instruction.rt();
	let rs  = instruction.rs();
	let imm = instruction.imm();

	//println!("ANDI ${}, ${}, 0x{:04x}", rt, rs, imm);

	let v = cpu.reg(rs) & imm;
	cpu.set_reg(rt, v);
}

pub fn op_ori(cpu: &mut CPU, instruction: Instruction) {
	let rt  = instruction.rt();
	let rs  = instruction.rs();
	let imm = instruction.imm();

	//println!("ORI ${}, ${}, 0x{:04x}", rt, rs, imm);

	let v = cpu.reg(rs) | imm;
	cpu.set_reg(rt, v);
}

pub fn op_xori(cpu: &mut CPU, instruction: Instruction) {
	let rt  = instruction.rt();
	let rs  = instruction.rs();
	let imm = instruction.imm();

	//println!("XORI ${}, ${}, 0x{:04x}", rt, rs, imm);

	let v = cpu.reg(rs) ^ imm;
	cpu.set_reg(rt, v);
}

pub fn op_lui(cpu: &mut CPU, instruction: Instruction) {
	let rt  = instruction.rt();
	let imm = instruction.imm();

	//println!("LUI ${}, 0x{:04x}", rt, imm);

	let v = imm << 16;
	cpu.set_reg(rt, v);
}

pub fn op_cop0(cpu: &mut CPU, instruction: Instruction) {
	match instruction.rs() {
		0b00000 => op_mfc0(cpu, instruction),
		0b00100 => op_mtc0(cpu, instruction),
		0b10000 => op_rfe(cpu, instruction),
		_ => op_reserved(cpu, instruction)	
	}
}

pub fn op_mfc0(cpu: &mut CPU, instruction: Instruction) {
	let cpu_reg = instruction.rt();
	let cop_reg = instruction.rd();

	//println!("MTC0 ${}, ${}", cop_reg, cpu_reg);

	let v = cpu.cop0_reg(cop_reg);
	cpu.set_reg(cpu_reg, v);
}

pub fn op_mtc0(cpu: &mut CPU, instruction: Instruction) {
	let cpu_reg = instruction.rt();
	let cop_reg = instruction.rd();

	//println!("MTC0 ${}, ${}", cop_reg, cpu_reg);

	let v = cpu.reg(cpu_reg);
	cpu.set_cop0_reg(cop_reg, v);
}

pub fn op_rfe(cpu: &mut CPU, instruction: Instruction) {
	if instruction.function() != 0b010000 {
		cpu.enter_exception(Exception::RESERVED)
	}

	//println!("RFE");
	cpu.exit_exception();
}

pub fn op_cop1(cpu: &mut CPU) {
	cpu.enter_exception(Exception::COPROCESSOR)
}

pub fn op_cop2(instruction: Instruction) {
	panic!("unimplemented GTE instruction 0x{:08x}", instruction.as_bytes());
}

pub fn op_cop3(cpu: &mut CPU) {
	cpu.enter_exception(Exception::COPROCESSOR)
}

pub fn op_lb(cpu: &mut CPU, instruction: Instruction) {
	let rt     = instruction.rt();
	let rs     = instruction.rs();
	let offset = instruction.imm_se();

	//println!("LB ${}, 0x{:04x}(${})", rt, offset, rs);

	let addr = cpu.reg(rs).wrapping_add(offset);
	let v = (cpu.load8(addr) as i8) as u32;
	cpu.set_reg(rt, v);
}

pub fn op_lh(cpu: &mut CPU, instruction: Instruction) {
	let rt     = instruction.rt();
	let rs     = instruction.rs();
	let offset = instruction.imm_se();

	//println!("LH ${}, 0x{:04x}(${})", rt, offset, rs);

	let addr = cpu.reg(rs).wrapping_add(offset);
	let v = (cpu.load16(addr) as i16) as u32;
	cpu.set_reg(rt, v);
}

pub fn op_lwl(cpu: &mut CPU, instruction: Instruction) {
	let rt  = instruction.rt();
	let rs  = instruction.rs();
	let offset = instruction.imm_se();

	//println!("LWL ${}, 0x{:04x}(${})", rt, offset, rs);

	let addr = cpu.reg(rs).wrapping_add(offset);

	let mut v = cpu.load_delay_regs[rt];

	let addr_aligned = addr & !3;
	let word_aligned = cpu.load32(addr_aligned);

	v = match addr & 3 {
		0 => (v & 0x00ffffff) | (word_aligned << 24),
		1 => (v & 0x0000ffff) | (word_aligned << 16),
		2 => (v & 0x000000ff) | (word_aligned <<  8),
		3 => (v & 0x00000000) | (word_aligned <<  0),
		_ => unreachable!(),
	};

	cpu.set_reg(rt, v);
}

pub fn op_lw(cpu: &mut CPU, instruction: Instruction) {
	let rt  = instruction.rt();
	let rs  = instruction.rs();
	let offset = instruction.imm_se();

	//println!("LW ${}, 0x{:04x}(${})", rt, offset, rs);

	let addr = cpu.reg(rs).wrapping_add(offset);
	let v = cpu.load32(addr);
	cpu.set_reg(rt, v);
}

pub fn op_lbu(cpu: &mut CPU, instruction: Instruction) {
	let rt  = instruction.rt();
	let rs  = instruction.rs();
	let offset = instruction.imm_se();

	//println!("LBU ${}, 0x{:04x}(${})", rt, offset, rs);

	let addr = cpu.reg(rs).wrapping_add(offset);
	let v = cpu.load8(addr) as u32;
	cpu.set_reg(rt, v);
}

pub fn op_lhu(cpu: &mut CPU, instruction: Instruction) {
	let rt  = instruction.rt();
	let rs  = instruction.rs();
	let offset = instruction.imm_se();

	//println!("LHU ${}, 0x{:04x}(${})", rt, offset, rs);

	let addr = cpu.reg(rs).wrapping_add(offset);
	let v = cpu.load16(addr) as u32;
	cpu.set_reg(rt, v);
}

pub fn op_lwr(cpu: &mut CPU, instruction: Instruction) {
	let rt  = instruction.rt();
	let rs  = instruction.rs();
	let offset = instruction.imm_se();

	//println!("LWR ${}, 0x{:04x}(${})", rt, offset, rs);

	let addr = cpu.reg(rs).wrapping_add(offset);

	let mut v = cpu.load_delay_regs[rt];

	let addr_aligned = addr & !3;
	let word_aligned = cpu.load32(addr_aligned);

	v = match addr & 3 {
		0 => (v & 0x00000000) | (word_aligned >>  0),
		1 => (v & 0xff000000) | (word_aligned >>  8),
		2 => (v & 0xffff0000) | (word_aligned >> 16),
		3 => (v & 0xffffff00) | (word_aligned >> 24),
		_ => unreachable!(),
	};

	cpu.set_reg(rt, v);
}

pub fn op_sb(cpu: &mut CPU, instruction: Instruction) {
	let rt  = instruction.rt();
	let rs  = instruction.rs();
	let offset = instruction.imm_se();

	//println!("SB ${}, 0x{:04x}(${})", rt, offset, rs);

	if cpu.cop0.isolate_cache() {
		return;
	}

	let addr = cpu.reg(rs).wrapping_add(offset);
	let v = cpu.reg(rt);
	cpu.store8(addr, v as u8);
}

pub fn op_sh(cpu: &mut CPU, instruction: Instruction) {
	let rt  = instruction.rt();
	let rs  = instruction.rs();
	let offset = instruction.imm_se();

	//println!("SH ${}, 0x{:04x}(${})", rt, offset, rs);

	if cpu.cop0.isolate_cache() {
		return;
	}

	let addr = cpu.reg(rs).wrapping_add(offset);
	let v = cpu.reg(rt);
	cpu.store16(addr, v as u16);
}

pub fn op_swl(cpu: &mut CPU, instruction: Instruction) {
	let rt  = instruction.rt();
	let rs  = instruction.rs();
	let offset = instruction.imm_se();

	//println!("SWL ${}, 0x{:04x}(${})", rt, offset, rs);

	if cpu.cop0.isolate_cache() {
		return;
	}

	let addr = cpu.reg(rs).wrapping_add(offset);
	let v = cpu.reg(rt);

	let addr_aligned = addr & !3;
	let mut memory = cpu.load32(addr_aligned);

	memory = match addr & 3 {
		0 => (memory & 0xffffff00) | (v >> 24),
		1 => (memory & 0xffff0000) | (v >> 16),
		2 => (memory & 0xff000000) | (v >>  8),
		3 => (memory & 0x00000000) | (v >>  0),
		_ => unreachable!(),
	};

	cpu.store32(addr_aligned, memory);
}

pub fn op_sw(cpu: &mut CPU, instruction: Instruction) {
	let rt  = instruction.rt();
	let rs  = instruction.rs();
	let offset = instruction.imm_se();

	//println!("SW ${}, 0x{:04x}(${})", rt, offset, rs);

	if cpu.cop0.isolate_cache() {
		return;
	}

	let addr = cpu.reg(rs).wrapping_add(offset);
	let v = cpu.reg(rt);
	cpu.store32(addr, v);
}

pub fn op_swr(cpu: &mut CPU, instruction: Instruction) {
	let rt  = instruction.rt();
	let rs  = instruction.rs();
	let offset = instruction.imm_se();

	//println!("SWR ${}, 0x{:04x}(${})", rt, offset, rs);

	if cpu.cop0.isolate_cache() {
		return;
	}

	let addr = cpu.reg(rs).wrapping_add(offset);
	let v = cpu.reg(rt);

	let addr_aligned = addr & !3;
	let mut memory = cpu.load32(addr_aligned);

	memory = match addr & 3 {
		0 => (memory & 0x00000000) | (v <<  0),
		1 => (memory & 0x000000ff) | (v <<  8),
		2 => (memory & 0x0000ffff) | (v << 16),
		3 => (memory & 0x00ffffff) | (v << 24),
		_ => unreachable!(),
	};

	cpu.store32(addr_aligned, memory);
}

pub fn op_lwc0(cpu: &mut CPU) {
	cpu.enter_exception(Exception::COPROCESSOR)
}

pub fn op_lwc1(cpu: &mut CPU) {
	cpu.enter_exception(Exception::COPROCESSOR)
}

pub fn op_lwc2(instruction: Instruction) {
	panic!("unimplemented GTE LWC 0x{:08x}", instruction.as_bytes());
}

pub fn op_lwc3(cpu: &mut CPU) {
	cpu.enter_exception(Exception::COPROCESSOR)
}

pub fn op_swc0(cpu: &mut CPU) {
	cpu.enter_exception(Exception::COPROCESSOR)
}

pub fn op_swc1(cpu: &mut CPU) {
	cpu.enter_exception(Exception::COPROCESSOR)
}

pub fn op_swc2(instruction: Instruction) {
	panic!("unimplemented GTE SWC 0x{:08x}", instruction.as_bytes());
}

pub fn op_swc3(cpu: &mut CPU) {
	cpu.enter_exception(Exception::COPROCESSOR)
}

pub fn op_reserved(cpu: &mut CPU, instruction: Instruction) {
	println!("Reserved instruction 0x{:08x}", instruction.as_bytes());
	cpu.enter_exception(Exception::RESERVED)
}