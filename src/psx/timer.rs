use std::cell::RefCell;
use std::rc::Rc;

use super::interrupt::{Interrupt, InterruptRegister};

pub struct Counter {
    value: u32,
    mode: u32,
    target: u32,

    div8counter: usize,

    interrupt: Rc<RefCell<InterruptRegister>>,
}

impl Counter {
    pub fn new(interrupt: Rc<RefCell<InterruptRegister>>) -> Counter {
        Counter {
            value: 0,
            mode: 0,
            target: 0,

            div8counter: 0,

            interrupt: interrupt,
        }
    }

    pub fn tick(&mut self, number: usize, clocks: usize) {
        let timer = match number {
            0 => Interrupt::Tmr0,
            1 => Interrupt::Tmr1,
            2 => Interrupt::Tmr2,
            _ => unreachable!(),
        };

        let mut irq = false;
        let mut i = self.interrupt.borrow_mut();

        let old_value = self.value;
        self.value += clocks as u32;

        if (old_value < self.target) && (self.value >= self.target) {
            self.mode |= 0x800;

            if (self.mode & 0x8) != 0 {
                self.value = 0;
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
            i.set_interrupt(timer);
        }
    }
}

pub struct Timer {
    counter: [Counter; 3],

    vblank: bool,
    hblank: bool,
}

impl Timer {
    pub fn new(interrupt: Rc<RefCell<InterruptRegister>>) -> Timer {
        Timer {
            counter: [Counter::new(interrupt.clone()), Counter::new(interrupt.clone()), Counter::new(interrupt)],

            vblank: false,
            hblank: false,
        }
    }

    pub fn read(&mut self, address: u32) -> u32 {
        let index = ((address & 0x30) >> 4) as usize;
        let section = address & 0xf;

        let counter = &mut self.counter[index];

        match section {
            0 => counter.value & 0xffff,
            4 => {
                let mode = counter.mode;
                counter.mode &= 0x7ff;
                mode
            },
            8 => counter.target & 0xffff,
            _ => panic!("[TIMER] [ERROR] Read from unrecognised address {:#x}", address),
        }
    }

    pub fn write(&mut self, address: u32, value: u32) {
        let index = ((address & 0x30) >> 4) as usize;
        let section = address & 0xf;

        let counter = &mut self.counter[index];

        match section {
            0 => counter.value = value & 0xffff,
            4 => {
                counter.mode = value & 0x3ff;
                counter.mode |= 0x400;
                counter.value = 0;
            },
            8 => counter.target = value & 0xffff,
            _ => panic!("[TIMER] [ERROR] Read from unrecognised address {:#x}", address),
        }
    }

    pub fn tick0(&mut self, clocks: usize) {
        let counter = &mut self.counter[0];

        if (counter.mode & 0x100) == 0 {
            counter.tick(0, clocks);
        }
    }

    pub fn tick1(&mut self, clocks: usize) {
        let counter = &mut self.counter[1];

        if (counter.mode & 0x100) == 0 {
            counter.tick(1, clocks);
        }
    }

    pub fn tick2(&mut self, mut clocks: usize) {
        let counter = &mut self.counter[2];

        if (counter.mode & 0x200) != 0 {
            counter.div8counter += clocks;
            clocks = counter.div8counter >> 3;
            counter.div8counter &= 0x7;
        }

        if (counter.mode & 0x1) != 0 {
            clocks = 0;
        }

        counter.tick(2, clocks);
    }

    pub fn tick_dotclock(&mut self, clocks: usize) {
        let counter = &mut self.counter[0];

        if (counter.mode & 0x100) != 0 {
            counter.tick(0, clocks);
        }
    }

    pub fn tick_hblank(&mut self) {
        let counter = &mut self.counter[1];

        if (counter.mode & 0x100) != 0 {
            counter.tick(1, 1);
        }
    }

    pub fn set_vblank(&mut self, state: bool) {
        self.vblank = state;
    }

    pub fn set_hblank(&mut self, state: bool) {
        self.hblank = state;
    }
}