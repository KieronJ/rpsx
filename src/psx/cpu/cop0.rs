use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq)]
pub enum Exception {
    Interrupt = 0,
    AddrLoad = 4,
    AddrStore = 5,
    IBusError = 6,
    DBusError = 7,
    Syscall = 8,
    Breakpoint = 9,
    Reserved = 10,
    Overflow = 12,
}

#[derive(Deserialize, Serialize)]
struct Dcic {
    trap: bool,
    user_debug: bool,
    kernel_debug: bool,
    trace: bool,
    data_write: bool,
    data_read: bool,
    data_breakpoint: bool,
    code_breakpoint: bool,
    master_debug: bool,
    hit_trace: bool,
    hit_write: bool,
    hit_read: bool,
    hit_data: bool,
    hit_code: bool,
    hit_debug: bool,
}

impl Dcic {
    pub fn new() -> Dcic {
        Dcic {
            trap: false,
            user_debug: false,
            kernel_debug: false,
            trace: false,
            data_write: false,
            data_read: false,
            data_breakpoint: false,
            code_breakpoint: false,
            master_debug: false,
            hit_trace: false,
            hit_write: false,
            hit_read: false,
            hit_data: false,
            hit_code: false,
            hit_debug: false,
        }
    }

    pub fn reset(&mut self) {
        self.trap = false;
        self.user_debug = false;
        self.kernel_debug = false;
        self.trace = false;
        self.data_write = false;
        self.data_read = false;
        self.data_breakpoint = false;
        self.code_breakpoint = false;
        self.master_debug = false;
        self.hit_trace = false;
        self.hit_write = false;
        self.hit_read = false;
        self.hit_data = false;
        self.hit_code = false;
        self.hit_debug = false;
    }

    pub fn read(&self) -> u32 {
        (self.trap as u32) << 31
        | (self.user_debug as u32) << 30
        | (self.kernel_debug as u32) << 29
        | (self.trace as u32) << 28
        | (self.data_write as u32) << 27
        | (self.data_read as u32) << 26
        | (self.data_breakpoint as u32) << 25
        | (self.code_breakpoint as u32) << 24
        | (self.master_debug as u32) << 23
        | (self.hit_trace as u32) << 5
        | (self.hit_write as u32) << 4
        | (self.hit_read as u32) << 3
        | (self.hit_data as u32) << 2
        | (self.hit_code as u32) << 1
        | (self.hit_debug as u32)
    }

    pub fn write(&mut self, value: u32) {
        self.trap = (value & 0x8000_0000) != 0;
        self.user_debug = (value & 0x4000_0000) != 0;
        self.kernel_debug = (value & 0x2000_0000) != 0;
        self.trace = (value & 0x1000_0000) != 0;
        self.data_write = (value & 0x800_0000) != 0;
        self.data_read = (value & 0x400_0000) != 0;
        self.data_breakpoint = (value & 0x200_0000) != 0;
        self.code_breakpoint = (value & 0x100_0000) != 0;
        self.master_debug = (value & 0x80_0000) != 0;
        self.hit_trace = (value & 0x20) != 0;
        self.hit_write = (value & 0x10) != 0;
        self.hit_read = (value & 0x8) != 0;
        self.hit_data = (value & 0x4) != 0;
        self.hit_code = (value & 0x2) != 0;
        self.hit_debug = (value & 0x1) != 0;
    }
}

#[derive(Deserialize, Serialize)]
struct Status {
    coprocessor_usability: [bool; 4],
    reverse_endianness: bool,
    bootstrap_exception_vector: bool,
    tlb_shutdown: bool,
    parity_error: bool,
    cache_miss: bool,
    parity_zero: bool,
    swap_caches: bool,
    isolate_cache: bool,
    interrupt_mask: u8,
    kernel_user_old: bool,
    interrupt_enable_old: bool,
    kernel_user_previous: bool,
    interrupt_enable_previous: bool,
    kernel_user_current: bool,
    interrupt_enable_current: bool,
}

impl Status {
    pub fn new() -> Status {
        Status {
            coprocessor_usability: [false; 4],
            reverse_endianness: false,
            bootstrap_exception_vector: false,
            tlb_shutdown: false,
            parity_error: false,
            cache_miss: false,
            parity_zero: false,
            swap_caches: false,
            isolate_cache: false,
            interrupt_mask: 0,
            kernel_user_old: false,
            interrupt_enable_old: false,
            kernel_user_previous: false,
            interrupt_enable_previous: false,
            kernel_user_current: false,
            interrupt_enable_current: false,
        }
    }

    pub fn reset(&mut self) {
        self.coprocessor_usability = [false; 4];
        self.reverse_endianness = false;
        self.bootstrap_exception_vector = true;
        self.tlb_shutdown = true;
        self.parity_error = false;
        self.cache_miss = false;
        self.parity_zero = false;
        self.swap_caches = false;
        self.isolate_cache = false;
        self.interrupt_mask = 0;
        self.kernel_user_old = false;
        self.interrupt_enable_old = false;
        self.kernel_user_previous = false;
        self.interrupt_enable_previous = false;
        self.kernel_user_current = false;
        self.interrupt_enable_current = false;
    }

    pub fn read(&self) -> u32 {
        (self.coprocessor_usability[3] as u32) << 31
        | (self.coprocessor_usability[2] as u32) << 30
        | (self.coprocessor_usability[1] as u32) << 29
        | (self.coprocessor_usability[0] as u32) << 28
        | (self.reverse_endianness as u32) << 25
        | (self.bootstrap_exception_vector as u32) << 22
        | (self.tlb_shutdown as u32) << 21
        | (self.parity_error as u32) << 20
        | (self.cache_miss as u32) << 19
        | (self.parity_zero as u32) << 18
        | (self.swap_caches as u32) << 17
        | (self.isolate_cache as u32) << 16
        | (self.interrupt_mask as u32) << 8
        | (self.kernel_user_old as u32) << 5
        | (self.interrupt_enable_old as u32) << 4
        | (self.kernel_user_previous as u32) << 3
        | (self.interrupt_enable_previous as u32) << 2
        | (self.kernel_user_current as u32) << 1
        | (self.interrupt_enable_current as u32)
    }

    pub fn write(&mut self, value: u32) {
        self.coprocessor_usability[3] = (value & 0x8000_0000) != 0;
        self.coprocessor_usability[2] = (value & 0x4000_0000) != 0;
        self.coprocessor_usability[1] = (value & 0x2000_0000) != 0;
        self.coprocessor_usability[0] = (value & 0x1000_0000) != 0;
        self.reverse_endianness = (value & 0x0200_0000) != 0;
        self.bootstrap_exception_vector = (value & 0x0040_0000) != 0;
        self.tlb_shutdown = (value & 0x0020_0000) != 0;
        self.parity_error = (value & 0x0010_0000) != 0;
        self.cache_miss = (value & 0x0008_0000) != 0;
        self.parity_zero = (value & 0x0004_0000) != 0;
        self.swap_caches = (value & 0x0002_0000) != 0;
        self.isolate_cache = (value & 0x0001_0000) != 0;
        self.interrupt_mask = (value >> 8) as u8;
        self.kernel_user_old = (value & 0x0000_0020) != 0;
        self.interrupt_enable_old = (value & 0x0000_0010) != 0;
        self.kernel_user_previous = (value & 0x0000_0008) != 0;
        self.interrupt_enable_previous = (value & 0x0000_0004) != 0;
        self.kernel_user_current = (value & 0x0000_0002) != 0;
        self.interrupt_enable_current = (value & 0x0000_0001) != 0;
    }

    pub fn enter_exception(&mut self) {
        self.kernel_user_old = self.kernel_user_previous;
        self.interrupt_enable_old = self.interrupt_enable_previous;

        self.kernel_user_previous = self.kernel_user_current;
        self.interrupt_enable_previous = self.interrupt_enable_current;

        self.kernel_user_current = false;
        self.interrupt_enable_current = false;
    }

    pub fn leave_exception(&mut self) {
        self.kernel_user_current = self.kernel_user_previous;
        self.interrupt_enable_current = self.interrupt_enable_previous;

        self.kernel_user_previous = self.kernel_user_old;
        self.interrupt_enable_previous = self.interrupt_enable_old;
    }
}

#[derive(Deserialize, Serialize)]
struct Cause {
    branch_delay: bool,
    branch_taken: bool,
    coprocessor_exception: u8,
    interrupt_pending: u8,
    exception_code: u8,
}

impl Cause {
    pub fn new() -> Cause {
        Cause {
            branch_delay: false,
            branch_taken: false,
            coprocessor_exception: 0,
            interrupt_pending: 0,
            exception_code: 0,
        }
    }

    pub fn reset(&mut self) {
        self.branch_delay = false;
        self.branch_taken = false;
        self.coprocessor_exception = 0;
        self.interrupt_pending = 0;
        self.exception_code = 0;
    }

    pub fn read(&self) -> u32 {
        (self.branch_delay as u32) << 31
        | (self.branch_taken as u32) << 30
        | ((self.coprocessor_exception & 0x03) as u32) << 28
        | (self.interrupt_pending as u32) << 8
        | ((self.exception_code & 0x1f) as u32) << 2
    }

    pub fn write(&mut self, value: u32) {
        self.interrupt_pending &= !0x03;
        self.interrupt_pending |= ((value >> 8) & 0x03) as u8;
    }

    pub fn set_interrupt_bit(&mut self) {
        self.interrupt_pending |= 0x4;
    }

    pub fn clear_interrupt_bit(&mut self) {
        self.interrupt_pending &= !0x4;
    }

    pub fn enter_exception(&mut self, exception: Exception, bd: bool, bt: bool, coprocessor: u8) {
        self.exception_code = exception as u8;
        self.coprocessor_exception = coprocessor;
        self.branch_delay = bd;
        self.branch_taken = bt;
    }
}

#[derive(Deserialize, Serialize)]
pub struct Cop0 {
    bpc: u32,
    bda: u32,
    jumpdest: u32,
    dcic: Dcic,
    bad_vaddr: u32,
    bdam: u32,
    bpcm: u32,
    status: Status,
    cause: Cause,
    epc: u32,
}

impl Cop0 {
    pub fn new() -> Cop0 {
        Cop0 {
            bpc: 0,
            bda: 0,
            jumpdest: 0,
            dcic: Dcic::new(),
            bad_vaddr: 0,
            bdam: 0,
            bpcm: 0,
            status: Status::new(),
            cause: Cause::new(),
            epc: 0,
        }
    }

    pub fn reset(&mut self) {
        self.dcic.reset();
        self.bpcm = 0xffff_ffff;
        self.bdam = 0xffff_ffff;
        self.status.reset();
        self.cause.reset();
    }

    pub fn read(&self, index: usize) -> u32 {
        match index {
            3 => self.bpc,
            5 => self.bda,
            6 => self.jumpdest,
            7 => self.dcic.read(),
            8 => self.bad_vaddr,
            9 => self.bdam,
            11 => self.bpcm,
            12 => self.status.read(),
            13 => self.cause.read(),
            14 => self.epc,
            15 => 0x0000_0002,
            _ => panic!(
                "[COP0] [ERROR] Read from unimplemented Cop0 register {}",
                index
            ),
        }
    }

    pub fn write(&mut self, index: usize, value: u32) {
        match index {
            3 => {
                self.bpc = value;
                //println!("Setting BPC to 0x{:08x}", self.bpc);
            },
            5 => {
                self.bda = value;
                //println!("Setting BDA to 0x{:08x}", self.bda);
            },
            6 => (),
            7 => self.dcic.write(value),
            9 => {
                self.bdam = value;
                //println!("Setting BDAM to 0x{:08x}", self.bdam);
            },
            11 => {
                self.bpcm = value;
                //println!("Setting BPCM to 0x{:08x}", self.bpcm);
            },
            12 => self.status.write(value),
            13 => self.cause.write(value),
            _ => panic!(
                "[COP0] [ERROR] Write to unimplemented Cop0 register {}",
                index
            ),
        }
    }

    pub fn enter_exception(&mut self, epc: u32, exception: Exception, bd: bool, bt: bool, coprocessor: u8) {
        self.epc = epc;
        self.status.enter_exception();
        self.cause.enter_exception(exception, bd, bt, coprocessor);
    }

    pub fn leave_exception(&mut self) {
        self.status.leave_exception();
    }

    pub fn exception_vectors(&self) -> bool {
        self.status.bootstrap_exception_vector
    }

    pub fn set_bad_vaddr(&mut self, value: u32) {
        self.bad_vaddr = value;
    }

    pub fn iec(&self) -> bool {
        self.status.interrupt_enable_current
    }

    pub fn im(&self) -> bool {
        (self.status.interrupt_mask & self.cause.interrupt_pending) != 0
    }

    pub fn set_jumpdest(&mut self, value: u32) {
        self.jumpdest = value;
    }

    pub fn test_code(&mut self, test: u32) -> bool {
        if !self.dcic.master_debug {
            return false;
        }

        let kernel = test >= 0x8000_0000;
        let user = !kernel;

        if !(self.dcic.kernel_debug && kernel)
            && !(self.dcic.user_debug && user) {
            return false;
        }

        if !self.dcic.code_breakpoint {
            return false;
        }

        if ((test ^ self.bpc) & self.bpcm) != 0 {
            return false;
        }

        self.dcic.hit_debug = true;
        self.dcic.hit_code = true;

        self.dcic.trap
    }

    pub fn test_read(&mut self, test: u32) -> bool {
        if !self.dcic.master_debug {
            return false;
        }

        let kernel = test >= 0x8000_0000;
        let user = !kernel;

        if !(self.dcic.kernel_debug && kernel)
            && !(self.dcic.user_debug && user) {
            return false;
        }

        if !self.dcic.data_breakpoint {
            return false;
        }

        if !self.dcic.data_read {
            return false;
        }

        if ((test ^ self.bda) & self.bdam) != 0 {
            return false;
        }

        self.dcic.hit_debug = true;
        self.dcic.hit_data = true;
        self.dcic.hit_read = true;

        self.dcic.trap
    }

    pub fn test_write(&mut self, test: u32) -> bool {
        if !self.dcic.master_debug {
            return false;
        }

        //if test == 0x801f0001 {
        //    println!("BDA:\t0x{:08x}", self.bda);
        //    println!("BDAM:\t0x{:08x}", self.bdam);
        //    println!("TEST:\t0x{:08x}", test);
        //}

        let kernel = test >= 0x8000_0000;
        let user = !kernel;

        if !(self.dcic.kernel_debug && kernel)
            && !(self.dcic.user_debug && user) {
            return false;
        }

        //if test == 0x801f0001 {
        //    println!("a");
        //}

        if !self.dcic.data_breakpoint {
            return false;
        }

        if !self.dcic.data_write {
            return false;
        }

        if ((test ^ self.bda) & self.bdam) != 0 {
            return false;
        }

        self.dcic.hit_debug = true;
        self.dcic.hit_data = true;
        self.dcic.hit_write = true;

        self.dcic.trap
    }

    pub fn set_interrupt_bit(&mut self) {
        self.cause.set_interrupt_bit();
    }

    pub fn clear_interrupt_bit(&mut self) {
        self.cause.clear_interrupt_bit();
    }

    pub fn isolate_cache(&self) -> bool {
        self.status.isolate_cache
    }
}