use std::cmp;

use serde::{Deserialize, Serialize};

use crate::util::{clip, f32_to_i16, i16_to_f32};

use crate::psx::adpcm::ADPCM_FILTERS;

use super::adsr::{Adsr, AdsrState};
use super::gauss::GAUSS_TABLE;
use super::volume::Volume;
use super::SpuRam;

pub const VOICE_SIZE: usize = 0x10;
pub const NR_SAMPLES: usize = 28;

#[derive(Clone, Copy, Deserialize, Serialize)]
pub struct Voice {
    counter: usize,

    volume: Volume,

    pitch: u16,
    pub modulator: i16,

    start_address: u32,
    repeat_address: u32,
    current_address: u32,

    repeat_address_written: bool,

    endx: bool,
    reverb: bool,
    noise: bool,

    adsr: Adsr,

    samples: [i16; NR_SAMPLES],
    prev_samples: [i16; 2],
    last_samples: [i16; 4],
}

impl Voice {
    pub fn new() -> Voice {
        Voice {
            counter: 0,

            volume: Volume::default(),

            pitch: 0,
            modulator: 0,

            start_address: 0,
            repeat_address: 0,
            current_address: 0,

            repeat_address_written: false,

            endx: false,
            reverb: false,
            noise: false,

            adsr: Adsr::default(),

            samples: [0; NR_SAMPLES],
            prev_samples: [0; 2],
            last_samples: [0; 4],
        }
    }

    pub fn from_address(address: u32) -> (usize, usize) {
        let voice = (address & 0x1f0) >> 4;
        let offset = address & 0xf;

        (voice as usize, offset as usize)
    }

    pub fn disabled(&self) -> bool {
        return self.adsr.state == AdsrState::Disabled;
    }

    pub fn reverb_enabled(&self) -> bool {
        self.reverb
    }

    pub fn key_on(&mut self) {
        self.adsr.state = AdsrState::Attack;
        self.adsr.volume = 0;
        self.adsr.cycles = 0;

        self.current_address = self.start_address;

        if !self.repeat_address_written {
            self.repeat_address = self.start_address;
        }

        self.repeat_address_written = false;
    }

    pub fn key_off(&mut self) {
        self.adsr.state = AdsrState::Release;
        self.adsr.cycles = 0;
    }

    pub fn endx(&mut self) -> bool {
        let endx = self.endx;
        self.endx = false;
        endx
    }

    pub fn set_noise(&mut self, state: bool) {
        self.noise = state;
    }

    pub fn echo_on(&mut self) {
        self.reverb = true;
    }

    fn sample_index(&self) -> usize {
        self.counter >> 12
    }

    fn gauss_index(&self) -> usize {
        (self.counter & 0xff0) >> 4
    }

    fn get_sample(&self, index: isize) -> i16 {
        if index < 0 {
            self.last_samples[(index + 4) as usize]
        } else {
            self.samples[index as usize]
        }
    }

    fn interpolate(&self, index: isize) -> f32 {
        let gauss_index = self.gauss_index();

        let s1 = self.get_sample(index - 3) as i32;
        let s2 = self.get_sample(index - 2) as i32;
        let s3 = self.get_sample(index - 1) as i32;
        let s4 = self.get_sample(index - 0) as i32;

        let mut out = 0;
        out += (GAUSS_TABLE[0x0ff - gauss_index] * s1) >> 15;
        out += (GAUSS_TABLE[0x1ff - gauss_index] * s2) >> 15;
        out += (GAUSS_TABLE[0x100 + gauss_index] * s3) >> 15;
        out += (GAUSS_TABLE[0x000 + gauss_index] * s4) >> 15;

        i16_to_f32(out as i16)
    }

    pub fn get_samples(&mut self, noise: bool, noise_level: f32) -> (f32, f32) {
        let index = self.sample_index();

        self.adsr.update();

        let mut sample;

        if noise {
            sample = noise_level;
        } else {
            sample = self.interpolate(index as isize);
        }

        sample *= i16_to_f32(self.adsr.volume);

        self.modulator = f32_to_i16(sample);

        let left = sample * self.volume.l();
        let right = sample * self.volume.r();

        (left, right)
    }

    fn update_sample_index(&mut self) {
        let new = self.sample_index() - NR_SAMPLES;

        self.counter &= 0xfff;
        self.counter |= new << 12;
    }

    fn decode_samples(&mut self, ram: &mut SpuRam) {
        let header = ram.memory_read16(self.current_address);
        let flags = header >> 8;
        let filter = ((header & 0xf0) >> 4) as usize;
        let mut shift = header & 0xf;

        if shift > 12 {
            shift = 8;
        }

        if filter > 5 {
            println!("[SPU] [WARN] Invalid filter {}", filter);
        }

        if (flags & 0x4) != 0 {
            self.repeat_address = self.current_address;
        }

        for i in 0..7 {
            self.current_address += 2;
            self.current_address &= 0x7ffff;

            let mut samples = ram.memory_read16(self.current_address);

            for j in 0..4 {
                let mut sample = (samples << 12) as i16 as i32;
                sample >>= shift;

                let mut quant = 32;
                quant += self.prev_samples[0] as i32 * ADPCM_FILTERS[filter][0] as i32;
                quant -= self.prev_samples[1] as i32 * ADPCM_FILTERS[filter][1] as i32;

                sample = clip(sample + (quant / 64), -0x8000, 0x7fff);

                self.samples[i * 4 + j] = sample as i16;
                self.prev_samples[1] = self.prev_samples[0];
                self.prev_samples[0] = sample as i16;

                samples >>= 4;
            }
        }

        self.current_address += 2;
        self.current_address &= 0x7ffff;

        if (flags & 0x1) != 0 {
            self.endx = true;
            self.current_address = self.repeat_address;

            if (flags & 0x2) == 0 && !self.noise {
                self.key_off();
                self.adsr.volume = 0;
            }
        }
    }

    pub fn update(&mut self, ram: &mut SpuRam, modulate: bool, modulator: i16) {
        let mut step = self.pitch as u32;

        if modulate {
            let factor = (modulator as i32 + 0x8000) as u32;
            step = step as i16 as u32;
            step = (step * factor) >> 15;
            step &= 0xffff;
        }

        self.counter += cmp::min(step, 0x4000) as usize;

        self.reverb = false;

        if self.sample_index() >= NR_SAMPLES {
            self.update_sample_index();

            self.last_samples[0] = self.samples[24];
            self.last_samples[1] = self.samples[25];
            self.last_samples[2] = self.samples[26];
            self.last_samples[3] = self.samples[27];

            self.decode_samples(ram);
        }
    }

    pub fn read16(&self, offset: usize) -> u16 {
        assert!(offset < VOICE_SIZE);

        match offset {
            0x0 => (self.volume.left as u16) >> 1,
            0x2 => (self.volume.right as u16) >> 1,
            0x4 => self.pitch,
            0x6 => (self.start_address / 8) as u16,
            0x8 => self.adsr.config as u16,
            0xa => (self.adsr.config >> 16) as u16,
            0xc => self.adsr.volume as u16,
            0xe => (self.repeat_address / 8) as u16,
            _ => panic!(
                "[SPU] [ERROR] Read from invalid voice register: 0x{:x}",
                offset
            ),
        }
    }

    pub fn write16(&mut self, offset: usize, value: u16) {
        assert!(offset < VOICE_SIZE);

        match offset {
            0x0 => {
                if (value & 0x8000) != 0 {
                    println!("[SPU] [WARN] Sweep enabled for left channel");
                }

                self.volume.left = (value << 1) as i16;
            }
            0x2 => {
                if (value & 0x8000) != 0 {
                    println!("[SPU] [WARN] Sweep enabled for right channel");
                }

                self.volume.right = (value << 1) as i16;
            }
            0x4 => self.pitch = value,
            0x6 => self.start_address = (value as u32) * 8,
            0x8 => {
                self.adsr.config &= 0xffff0000;
                self.adsr.config |= value as u32;
            }
            0xa => {
                self.adsr.config &= 0xffff;
                self.adsr.config |= (value as u32) << 16;
            }
            0xc => self.adsr.volume = value as i16,
            0xe => {
                self.repeat_address = (value as u32) * 8;
                self.repeat_address_written = true;
            },
            _ => panic!(
                "[SPU] [ERROR] Write to invalid voice register: 0x{:x}",
                offset
            ),
        };
    }
}
