use std::cmp;

use super::intc::{Intc, Interrupt};

pub struct Counter {
    value: u32,
    mode: u32,
    target: u32,

    div8counter: usize,
}

impl Counter {
    pub fn new() -> Counter {
        Counter {
            value: 0,
            mode: 0,
            target: 0,

            div8counter: 0,
        }
    }

    pub fn tick(&mut self, intc: &mut Intc, number: usize, clocks: usize) {
        let timer = match number {
            0 => Interrupt::Tmr0,
            1 => Interrupt::Tmr1,
            2 => Interrupt::Tmr2,
            _ => unreachable!(),
        };

        let mut irq = false;

        if (self.mode & 0x8) != 0 && self.target == 0 && self.value == 0 {
            self.mode |= 0x800;

            if (self.mode & 0x8) != 0 {
                self.value %= cmp::max(1, self.target);
            }

            if (self.mode & 0x10) != 0 {
                intc.assert_irq(timer);
            }

            return;
        }

        let old_value = self.value;
        self.value += clocks as u32;

        if (old_value < self.target && self.value >= self.target)
            || (self.value >= self.target + 0x10000)
        {
            self.mode |= 0x800;

            if (self.mode & 0x8) != 0 {
                self.value %= cmp::max(1, self.target);
            }

            if (self.mode & 0x10) != 0 {
                irq = true;
            }
        }

        if self.value > 0xffff {
            self.mode |= 0x1000;

            self.value &= 0xffff;

            if (self.mode & 0x20) != 0 {
                irq = true;
            }
        }

        irq &= (self.mode & 0x400) != 0;

        if irq && ((self.mode & 0x80) != 0) {
            self.mode ^= 0x400;
        }

        if (self.mode & 0x40) == 0 {
            self.mode &= !0x400;
        }

        if irq {
            intc.assert_irq(timer);
        }
    }
}

pub struct Timers {
    counter: [Counter; 3],

    vblank: bool,
    hblank: bool,
}

impl Timers {
    pub fn new() -> Timers {
        Timers {
            counter: [
                Counter::new(),
                Counter::new(),
                Counter::new(),
            ],

            vblank: false,
            hblank: false,
        }
    }

    pub fn read(&mut self, address: u32) -> u32 {
        let index = ((address & 0x30) >> 4) as usize;
        let section = address & 0xc;

        let counter = &mut self.counter[index];

        match section {
            0 => counter.value,
            4 => {
                let mode = counter.mode;
                counter.mode &= !0x1000;

                if counter.value != counter.target {
                    counter.mode &= !0x800;
                }
                mode
            }
            8 => counter.target & 0xffff,
            _ => panic!(
                "[TIMER] [ERROR] Read from unrecognised address {:#x}",
                address
            ),
        }
    }

    pub fn write(&mut self, address: u32, value: u32) {
        let index = ((address & 0x30) >> 4) as usize;
        let section = address & 0xc;

        let counter = &mut self.counter[index];

        match section {
            0 => counter.value = value & 0xffff,
            4 => {
                counter.mode = (value & 0x3ff) | (counter.mode & 0x1c00);
                counter.mode |= 0x400;
                counter.value = 0;

                if (value & 0x1) != 0 {
                    println!(
                        "[TIMER] [INFO] Setting sync mode to {} for timer #{}",
                        (value & 0x6) >> 1,
                        index
                    );
                }

                if (value & 0x300) != 0 {
                    println!(
                        "[TIMER] [INFO] Setting clock source to {} for timer #{}",
                        (value & 0x300) >> 8,
                        index
                    );
                }
            }
            8 => counter.target = value & 0xffff,
            _ => panic!(
                "[TIMER] [ERROR] Read from unrecognised address {:#x}",
                address
            ),
        }
    }

    pub fn tick(&mut self, intc: &mut Intc, clocks: usize) {
        self.tick0(intc, clocks);
        self.tick1(intc, clocks);
        self.tick2(intc, clocks);
    }

    pub fn tick0(&mut self, intc: &mut Intc, clocks: usize) {
        let counter = &mut self.counter[0];

        if (counter.mode & 0x100) == 0 {
            counter.tick(intc, 0, clocks);
        }
    }

    pub fn tick1(&mut self, intc: &mut Intc, clocks: usize) {
        let counter = &mut self.counter[1];

        if (counter.mode & 0x7) == 0x7 {
            return;
        }

        if (counter.mode & 0x100) == 0 {
            counter.tick(intc, 1, clocks);
        }
    }

    pub fn tick2(&mut self, intc: &mut Intc, mut clocks: usize) {
        let counter = &mut self.counter[2];

        if (counter.mode & 0x200) != 0 {
            counter.div8counter += clocks;
            clocks = counter.div8counter >> 3;
            counter.div8counter &= 0x7;
        }

        if (counter.mode & 0x1) != 0 {
            clocks = 0;
        }

        counter.tick(intc, 2, clocks);
    }

    pub fn tick_dotclock(&mut self, intc: &mut Intc, clocks: usize) {
        let counter = &mut self.counter[0];

        if (counter.mode & 0x100) != 0 {
            counter.tick(intc, 0, clocks);
        }
    }

    pub fn tick_hblank(&mut self, intc: &mut Intc) {
        let counter = &mut self.counter[1];

        if (counter.mode & 0x7) == 0x7 {
            return;
        }

        if (counter.mode & 0x100) != 0 {
            counter.tick(intc, 1, 1);
        }
    }

    pub fn set_vblank(&mut self, state: bool) {
        self.vblank = state;

        if self.vblank && (self.counter[1].mode & 0x7) == 0x7 {
            self.counter[1].mode &= !0x1;
        }
    }

    pub fn set_hblank(&mut self, state: bool) {
        self.hblank = state;
    }
}
