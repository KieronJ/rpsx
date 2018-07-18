#[derive(PartialEq)]
pub enum Exception {
    Interrupt = 0,
    AddrLoad = 4,
    AddrStore = 5,
    Syscall = 8,
    Breakpoint = 9,
    Reserved = 10,
    Overflow = 12,
}

struct Cop0Status {
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

impl Cop0Status {
    pub fn new() -> Cop0Status
    {
        Cop0Status {
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

    pub fn reset(&mut self)
    {
        self.tlb_shutdown = true;
        self.bootstrap_exception_vector = true;
        self.kernel_user_current = false;
        self.interrupt_enable_current = false;
    }

    pub fn read(&self) -> u32
    {
        (self.coprocessor_usability[3] as u32)   << 31 |
        (self.coprocessor_usability[2] as u32)   << 30 |
        (self.coprocessor_usability[1] as u32)   << 29 |
        (self.coprocessor_usability[0] as u32)   << 28 |
        (self.reverse_endianness as u32)         << 25 |
        (self.bootstrap_exception_vector as u32) << 22 |
        (self.tlb_shutdown as u32)               << 21 |
        (self.parity_error as u32)               << 20 |
        (self.cache_miss as u32)                 << 19 |
        (self.parity_zero as u32)                << 18 |
        (self.swap_caches as u32)                << 17 |
        (self.isolate_cache as u32)              << 16 |
        (self.interrupt_mask as u32)             << 8  |
        (self.kernel_user_old as u32)            << 5  |
        (self.interrupt_enable_old as u32)       << 4  |
        (self.kernel_user_previous as u32)       << 3  |
        (self.interrupt_enable_previous as u32)  << 2  |
        (self.kernel_user_current as u32)        << 1  |
        (self.interrupt_enable_current as u32)
    }

    pub fn write(&mut self, value: u32)
    {
        self.coprocessor_usability[3]   = (value & 0x8000_0000) != 0;
        self.coprocessor_usability[2]   = (value & 0x4000_0000) != 0;
        self.coprocessor_usability[1]   = (value & 0x2000_0000) != 0;
        self.coprocessor_usability[0]   = (value & 0x1000_0000) != 0;
        self.reverse_endianness         = (value & 0x0200_0000) != 0;
        self.bootstrap_exception_vector = (value & 0x0040_0000) != 0;
        self.tlb_shutdown               = (value & 0x0020_0000) != 0;
        self.parity_error               = (value & 0x0010_0000) != 0;
        self.cache_miss                 = (value & 0x0008_0000) != 0;
        self.parity_zero                = (value & 0x0004_0000) != 0;
        self.swap_caches                = (value & 0x0002_0000) != 0;
        self.isolate_cache              = (value & 0x0001_0000) != 0;
        self.interrupt_mask             = (value >> 8) as u8;
        self.kernel_user_old            = (value & 0x0000_0020) != 0;
        self.interrupt_enable_old       = (value & 0x0000_0010) != 0;
        self.kernel_user_previous       = (value & 0x0000_0008) != 0;
        self.interrupt_enable_previous  = (value & 0x0000_0004) != 0;
        self.kernel_user_current        = (value & 0x0000_0002) != 0;
        self.interrupt_enable_current   = (value & 0x0000_0001) != 0;
    }

    pub fn enter_exception(&mut self)
    {
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

struct Cop0Cause {
    branch_delay: bool,
    coprocessor_exception: u8,
    interrupt_pending: u8,
    exception_code: u8,
}

impl Cop0Cause {
    pub fn new() -> Cop0Cause
    {
        Cop0Cause {
            branch_delay: false,
            coprocessor_exception: 0,
            interrupt_pending: 0,
            exception_code: 0,
        }
    }

    pub fn read(&self) -> u32
    {
        (self.branch_delay as u32)                   << 31 |
        ((self.coprocessor_exception & 0x03) as u32) << 28 |
        (self.interrupt_pending as u32)              << 8  |
        ((self.exception_code & 0x1f) as u32)        << 2
    }

    pub fn write(&mut self, value: u32)
    {
        self.interrupt_pending &= !0x03;
        self.interrupt_pending |= ((value >> 8) & 0x03) as u8;
    }

    pub fn set_interrupt_bit(&mut self) {
        self.interrupt_pending |= 0x4;
    }

    pub fn clear_interrupt_bit(&mut self) {
        self.interrupt_pending &= !0x4;
    }

    pub fn enter_exception(&mut self, exception: Exception, bd: bool) {
        self.exception_code = exception as u8;
        self.branch_delay = bd;
    }
}

pub struct Cop0 {
    bad_vaddr: u32,
    status: Cop0Status,
    cause: Cop0Cause,
    epc: u32,
}

impl Cop0 {
    pub fn new() -> Cop0 {
        Cop0 {
            bad_vaddr: 0,
            status: Cop0Status::new(),
            cause: Cop0Cause::new(),
            epc: 0,
        }
    }

    pub fn reset(&mut self, epc: u32) {
        self.status.reset();
        self.epc = epc;
    }

    pub fn read(&self, index: usize) -> u32 {
        match index {
            6 => 0,
            7 => 0,
            8 => self.bad_vaddr,
            9 => 0,
            12 => self.status.read(),
            13 => self.cause.read(),
            14 => self.epc,
            15 => 0x0000_0002,
            _ => panic!("[COP0] [ERROR] Read from unimplemented Cop0 register {}", index)
        }   
    }

    pub fn write(&mut self, index: usize, value: u32) {
        match index {
            3  => (),
            5  => (),
            6  => (),
            7  => (),
            9  => (),
            11 => (),
            12 => self.status.write(value),
            13 => self.cause.write(value),
            _ => panic!("[COP0] [ERROR] Write to unimplemented Cop0 register {}", index)
        }
    }

    pub fn enter_exception(&mut self, epc: u32, exception: Exception, bd: bool) {
        self.epc = epc;
        self.status.enter_exception();
        self.cause.enter_exception(exception, bd);
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