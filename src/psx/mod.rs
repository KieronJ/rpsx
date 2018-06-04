mod bus;
pub mod cpu;
mod display;
mod dma;
mod gpu;
pub mod interrupt;
mod rasteriser;
mod timer;

use self::bus::Bus;
use self::cpu::R3000A;
use self::interrupt::Interrupt;

pub struct System {
    cpu: R3000A,
}

impl System {
    pub fn new(bios: &str) -> System {
        let bus = Bus::new(bios);

        System {
            cpu: R3000A::new(bus),
        }
    }

    pub fn cpu(&mut self) -> &mut R3000A {
        &mut self.cpu
    }

    pub fn tick(&mut self) {
        if self.cpu.bus().timer0().tick() {
            self.set_interrupt(Interrupt::Tmr0);
        }

        if self.cpu.bus().timer1().tick() {
            self.set_interrupt(Interrupt::Tmr1);
        }

        if self.cpu.bus().timer2().tick() {
            self.set_interrupt(Interrupt::Tmr2);
        }
    }

    pub fn render_frame(&mut self) {
        self.cpu.bus().gpu().render_frame();
    }

    pub fn set_interrupt(&mut self, interrupt: Interrupt) {
        self.cpu.set_interrupt(interrupt);
    }

    pub fn reset(&mut self) {
        self.cpu.reset();
    }

    pub fn run(&mut self) {
        self.cpu.run();
    }
}