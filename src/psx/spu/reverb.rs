use std::cmp;

use crate::util::{clip, f32_to_i16, i16_to_f32};

use super::SpuRam;

#[derive(Debug, Default)]
pub struct Reverb {
    counter: usize,
    output: [f32; 2],

    buffer_address: u32,

    mbase: u32,

    dapf1: u32,
    dapf2: u32,

    viir: i16,
    vcomb1: i16,
    vcomb2: i16,
    vcomb3: i16,
    vcomb4: i16,
    vwall: i16,
    vapf1: i16,
    vapf2: i16,

    msame: [u32; 2],
    mcomb1: [u32; 2],
    mcomb2: [u32; 2],

    dsame: [u32; 2],

    mdiff: [u32; 2],
    mcomb3: [u32; 2],
    mcomb4: [u32; 2],

    ddiff: [u32; 2],

    mapf1: [u32; 2],
    mapf2: [u32; 2],

    vin: [i16; 2],
}

impl Reverb {
    pub fn calculate(&mut self, ram: &mut SpuRam, input: [f32; 2]) {
        self.counter = (self.counter + 1) % 2;

        if self.counter == 1 {
            return;
        }

        for i in 0..2 {
            let mut msame = input[i] * i16_to_f32(self.vin[i]);
            msame += self.read(ram, self.dsame[i]) * i16_to_f32(self.vwall);
            msame -= self.read(ram, self.msame[i] - 2);
            msame *= i16_to_f32(self.viir);
            msame += self.read(ram, self.msame[i] - 2);
            self.write(ram, self.msame[i], msame);

            let mut mdiff = input[i] * i16_to_f32(self.vin[i]);
            mdiff += self.read(ram, self.ddiff[1 - i]) * i16_to_f32(self.vwall);
            mdiff -= self.read(ram, self.mdiff[i] - 2);
            mdiff *= i16_to_f32(self.viir);
            mdiff += self.read(ram, self.mdiff[i] - 2);
            self.write(ram, self.mdiff[i], mdiff);

            self.output[i] = i16_to_f32(self.vcomb1) * self.read(ram, self.mcomb1[i]);
            self.output[i] += i16_to_f32(self.vcomb2) * self.read(ram, self.mcomb2[i]);
            self.output[i] += i16_to_f32(self.vcomb3) * self.read(ram, self.mcomb3[i]);
            self.output[i] += i16_to_f32(self.vcomb4) * self.read(ram, self.mcomb4[i]);

            self.output[i] -= i16_to_f32(self.vapf1) * self.read(ram, self.mapf1[i] - self.dapf1);
            self.write(ram, self.mapf1[i], self.output[i]);
            self.output[i] = self.output[i] * i16_to_f32(self.vapf1)
                + self.read(ram, self.mapf1[i] - self.dapf1);

            self.output[i] -= i16_to_f32(self.vapf2) * self.read(ram, self.mapf2[i] - self.dapf2);
            self.write(ram, self.mapf2[i], self.output[i]);
            self.output[i] = self.output[i] * i16_to_f32(self.vapf2)
                + self.read(ram, self.mapf2[i] - self.dapf2);
        }

        self.buffer_address = cmp::max(self.mbase, (self.buffer_address + 2) & 0x7fffe);
    }

    fn read(&self, ram: &mut SpuRam, address: u32) -> f32 {
        let sample = ram.memory_read16(self.calc_addr(address)) as i16;
        i16_to_f32(sample)
    }

    fn write(&self, ram: &mut SpuRam, address: u32, value: f32) {
        let sample = f32_to_i16(clip(value, -1.0, 1.0));
        ram.memory_write16(self.calc_addr(address), sample as u16);
    }

    fn calc_addr(&self, address: u32) -> u32 {
        let mut offset = self.buffer_address + address - self.mbase;
        offset %= 0x80000 - self.mbase;

        (self.mbase + offset) & 0x7fffe
    }

    pub fn output_l(&self) -> f32 {
        clip(self.output[0], -1.0, 1.0)
    }

    pub fn output_r(&self) -> f32 {
        clip(self.output[1], -1.0, 1.0)
    }

    pub fn get_base(&self) -> u16 {
        (self.mbase / 8) as u16
    }

    pub fn set_base(&mut self, value: u16) {
        self.mbase = (value as u32) * 8;
        self.buffer_address = (value as u32) * 8;
    }

    pub fn read16(&self, address: u32) -> u16 {
        match address {
            _ => panic!(
                "[SPU] [ERROR] Read from invalid reverb register: 0x{:08x}",
                address
            ),
        }
    }

    pub fn write16(&mut self, address: u32, value: u16) {
        match address {
            0x1f801dc0 => self.dapf1 = (value as u32) * 8,
            0x1f801dc2 => self.dapf2 = (value as u32) * 8,
            0x1f801dc4 => self.viir = value as i16,
            0x1f801dc6 => self.vcomb1 = value as i16,
            0x1f801dc8 => self.vcomb2 = value as i16,
            0x1f801dca => self.vcomb3 = value as i16,
            0x1f801dcc => self.vcomb4 = value as i16,
            0x1f801dce => self.vwall = value as i16,
            0x1f801dd0 => self.vapf1 = value as i16,
            0x1f801dd2 => self.vapf2 = value as i16,
            0x1f801dd4 => self.msame[0] = (value as u32) * 8,
            0x1f801dd6 => self.msame[1] = (value as u32) * 8,
            0x1f801dd8 => self.mcomb1[0] = (value as u32) * 8,
            0x1f801dda => self.mcomb1[1] = (value as u32) * 8,
            0x1f801ddc => self.mcomb2[0] = (value as u32) * 8,
            0x1f801dde => self.mcomb2[1] = (value as u32) * 8,
            0x1f801de0 => self.dsame[0] = (value as u32) * 8,
            0x1f801de2 => self.dsame[1] = (value as u32) * 8,
            0x1f801de4 => self.mdiff[0] = (value as u32) * 8,
            0x1f801de6 => self.mdiff[1] = (value as u32) * 8,
            0x1f801de8 => self.mcomb3[0] = (value as u32) * 8,
            0x1f801dea => self.mcomb3[1] = (value as u32) * 8,
            0x1f801dec => self.mcomb4[0] = (value as u32) * 8,
            0x1f801dee => self.mcomb4[1] = (value as u32) * 8,
            0x1f801df0 => self.ddiff[0] = (value as u32) * 8,
            0x1f801df2 => self.ddiff[1] = (value as u32) * 8,
            0x1f801df4 => self.mapf1[0] = (value as u32) * 8,
            0x1f801df6 => self.mapf1[1] = (value as u32) * 8,
            0x1f801df8 => self.mapf2[0] = (value as u32) * 8,
            0x1f801dfa => self.mapf2[1] = (value as u32) * 8,
            0x1f801dfc => self.vin[0] = value as i16,
            0x1f801dfe => self.vin[1] = value as i16,
            _ => panic!(
                "[SPU] [ERROR] Write to invalid reverb register: 0x{:08x}",
                address
            ),
        };
    }
}
