#[derive(Clone, Copy, PartialEq)]
enum TimerSource {
    Source0,
    Source1,
    Source2,
    Source3,
}

impl TimerSource {
    pub fn from(value: u32) -> TimerSource {
        match value & 0x03 {
            0 => TimerSource::Source0,
            1 => TimerSource::Source1,
            2 => TimerSource::Source2,
            3 => TimerSource::Source3,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy)]
enum TimerSync {
    Sync0,
    Sync1,
    Sync2,
    Sync3,
}

impl TimerSync {
    pub fn from(value: u32) -> TimerSync {
        match value & 0x03 {
            0 => TimerSync::Sync0,
            1 => TimerSync::Sync1,
            2 => TimerSync::Sync2,
            3 => TimerSync::Sync3,
            _ => unreachable!(),
        }
    }
}

struct TimerMode {
    reached_max: bool,
    reached_target: bool,
    irq: bool,
    clock_source: TimerSource,
    irq_pulse: bool,
    irq_repeat: bool,
    irq_on_max: bool,
    irq_on_target: bool,
    reset_on_target: bool,
    sync_mode: TimerSync,
    sync_enable: bool,
}

impl TimerMode {
    pub fn new() -> TimerMode {
        TimerMode {
            reached_max: false,
            reached_target: false,
            irq: false,
            clock_source: TimerSource::Source0,
            irq_pulse: false,
            irq_repeat: false,
            irq_on_max: false,
            irq_on_target: false,
            reset_on_target: false,
            sync_mode: TimerSync::Sync0,
            sync_enable: false,
        }
    }

    pub fn read(&mut self) -> u32 {
        let mut value = 0;

        value |= (self.reached_max as u32)     << 12;
        value |= (self.reached_target as u32)  << 11;
        value |= (self.irq as u32)             << 10;
        value |= (self.clock_source as u32)    << 8;
        value |= (self.irq_pulse as u32)       << 7;
        value |= (self.irq_repeat as u32)      << 6;
        value |= (self.irq_on_max as u32)      << 5;
        value |= (self.irq_on_target as u32)   << 4;
        value |= (self.reset_on_target as u32) << 3;
        value |= (self.sync_mode as u32)       << 1;
        value |= (self.sync_enable as u32)     << 0;

        self.reached_max = false;
        self.reached_target = false;

        value
    }

    pub fn write(&mut self, value: u32) {
        self.irq = true;
        self.clock_source = TimerSource::from(value >> 8);
        self.irq_pulse = (value & 0x80) != 0;
        self.irq_repeat = (value & 0x40) != 0;
        self.irq_on_max = (value & 0x20) != 0;
        self.irq_on_target = (value & 0x10) != 0;
        self.reset_on_target = (value & 0x8) != 0;
        self.sync_mode = TimerSync::from(value >> 1);
        self.sync_enable = (value & 0x1) != 0;

        if self.sync_enable {
            panic!("[TIMER] [ERROR] Unsupported synchronization mode.");
        }
    }
}

pub struct Timer {
    number: usize,

    value: u16,
    mode: TimerMode,
    target: u16,
}

impl Timer {
    pub fn new(number: usize) -> Timer {
        Timer {
            number: number,

            value: 0,
            mode: TimerMode::new(),
            target: 0,
        }
    }

    pub fn tick(&mut self) -> bool {
        self.value = self.value.wrapping_add(1);

        let mut irq = false;

        if self.value == 0 {
            self.mode.reached_max = true;

            if self.mode.irq_on_max {
                irq = true;
            }
        }

        if self.value == self.target {
            self.mode.reached_target = true;

            if self.mode.irq_on_target {
                irq = true;
            }

            if self.mode.reset_on_target {
                self.value = 0;
            }
        }

        irq &= self.mode.irq;

        if self.mode.irq_pulse {
            self.mode.irq = !self.mode.irq;
        }

        if !self.mode.irq_repeat {
            self.mode.irq = false;
        }

        irq
    }

    pub fn read_value(&self) -> u32 {
        self.value as u32
    }

    pub fn write_value(&mut self, value: u32) {
        self.value = value as u16;
    }

    pub fn read_mode(&mut self) -> u32 {
        self.mode.read()
    }

    pub fn write_mode(&mut self, value: u32) {
        self.mode.write(value);
        self.value = 0;
    }

    pub fn read_target(&self) -> u32 {
        self.target as u32
    }

    pub fn write_target(&mut self, value: u32) {
        self.target = value as u16;
    }
}