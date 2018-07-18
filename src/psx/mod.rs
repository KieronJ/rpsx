mod bus;
pub mod cpu;
mod cdrom;
mod controller;
mod display;
mod dma;
mod gpu;
mod interrupt;
mod joypad;
mod mdec;
mod rasteriser;
mod spu;
mod timer;

use self::bus::Bus;
use self::cpu::R3000A;
use self::interrupt::Interrupt;

pub struct System {
    cpu: R3000A,
}

impl System {
    pub fn new(bios_filepath: &str, game_filepath: &str) -> System {
        let bus = Bus::new(bios_filepath, game_filepath);

        System {
            cpu: R3000A::new(bus),
        }
    }

    pub fn cpu(&mut self) -> &mut R3000A {
        &mut self.cpu
    }

    pub fn tick(&mut self, clocks: usize) {
        if self.cpu.bus().cdrom().tick(clocks) {
            self.set_interrupt(Interrupt::Cdrom);
        }

        self.cpu.bus().tick_timers(clocks * 3);

        if self.cpu.bus().joypad().tick(clocks) {
            self.set_interrupt(Interrupt::Controller);
        }
    }

    pub fn check_dma_int(&mut self) {
        if self.cpu.bus().dma().check_interrupts() {
            self.set_interrupt(Interrupt::Dma);
        }
    }

    pub fn tick_gpu(&mut self, clocks: usize) {
        if self.cpu.bus().gpu().tick(clocks) {
            self.set_interrupt(Interrupt::Vblank);
        }

        if self.cpu.bus().gpu().irq() {
            self.set_interrupt(Interrupt::Gpu);
        }
    }

    pub fn set_interrupt(&mut self, interrupt: Interrupt) {
        self.cpu.bus().set_interrupt(interrupt);
    }

    pub fn reset(&mut self) {
        self.cpu.reset();
    }

    pub fn run(&mut self) {
        self.cpu.run();
    }
}