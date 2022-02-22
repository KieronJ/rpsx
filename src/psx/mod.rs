mod adpcm;
pub mod bus;
mod cdrom;
pub mod cpu;
mod exp2;
mod gpu;
mod intc;
mod mdec;
mod peripherals;
pub mod rasteriser;
mod scheduler;
mod spu;
mod timekeeper;
mod timers;

use std::fs::File;
use std::io;

use crate::gpu_viewer::GpuFrame;
use crate::util;

use self::bus::Bus;
use self::peripherals::controller::Controller;
use self::cpu::R3000A;
use self::timekeeper::Timekeeper;

pub struct System {
    pub running: bool,

    bus: Bus,
    cpu: R3000A,

    timekeeper: Timekeeper,
}

impl System {
    pub fn new(bios_filepath: &str, game_filepath: &str) -> System {
        System {
            running: true,

            bus: Bus::new(bios_filepath, game_filepath),
            cpu: R3000A::new(),

            timekeeper: Timekeeper::new(),
        }
    }

    pub fn reset(&mut self) {
        self.bus.reset();
        self.cpu.reset();

        self.timekeeper.reset();
    }

    pub fn run_frame(&mut self) {
        while !self.bus.gpu_mut().frame_complete() {
            while self.timekeeper.elapsed() < 128 {
                self.cpu.run(&mut self.bus, &mut self.timekeeper);
            }

            self.timekeeper.sync_all(&mut self.bus);
        }

        self.bus.peripherals().sync();
    }

    pub fn load_psexe(&mut self, filename: String) -> io::Result<()> {
        let mut file = File::open(filename)?;

        util::discard(&mut file, 0x10)?;

        self.cpu.pc = util::read_u32(&mut file)?;
        self.cpu.new_pc = self.cpu.pc + 4;

        self.cpu.regs[28] = util::read_u32(&mut file)?;

        let file_dest = util::read_u32(&mut file)? as usize;
        let file_size = util::read_u32(&mut file)? as usize;

        util::discard(&mut file, 0x10)?;

        self.cpu.regs[29] = util::read_u32(&mut file)? + util::read_u32(&mut file)?;
        self.cpu.regs[30] = self.cpu.regs[29];

        util::discard(&mut file, 0x7c8)?;

        let ram = self.bus.ram();

        for i in 0..file_size {
            ram[(file_dest + i) & 0x1fffff] = util::read_u8(&mut file)?;
        }

        Ok(())
    }

    pub fn get_audio_samples(&mut self) -> Vec<i16> {
        self.bus.spu().drain_samples()
    }

    pub fn get_controller(&mut self) -> &mut Controller {
        self.bus.peripherals().controller()
    }

    pub fn get_24bit(&self) -> bool {
        self.bus.gpu().get_24bit()
    }

    pub fn get_display_origin(&self) -> (u32, u32) {
        self.bus.gpu().get_display_origin()
    }

    pub fn get_display_size(&self) -> (u32, u32) {
        self.bus.gpu().get_display_size()
    }

    pub fn get_framebuffer(&self,
                           data: &mut [u8],
                           draw_full_vram: bool) {
        self.bus.gpu().get_framebuffer(data, draw_full_vram)
    }

    pub fn get_frame_data(&mut self) -> &mut GpuFrame {
        self.bus.gpu_mut().get_frame_data()
    }

    pub fn dump_vram(&self) {
        self.bus.gpu().dump_vram();
    }
}