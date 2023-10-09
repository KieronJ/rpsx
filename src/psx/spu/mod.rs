mod adsr;
mod gauss;
mod reverb;
mod voice;
mod volume;

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::util::{clip, f32_to_i16, i16_to_f32};

use super::intc::{Intc, Interrupt};

use self::reverb::Reverb;
use self::voice::Voice;
use self::volume::Volume;

const SPU_BUFFER_SIZE: usize = 32768;

const SPU_FIFO_SIZE: usize = 32;

const SPU_RAM_SIZE: usize = 0x80000;
const SPU_WORD_SIZE: usize = 2;

const SPU_NR_VOICES: usize = 24;

const NOISE_WAVE_TABLE: [isize; 64] = [
    1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0,
    0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 1, 0, 0, 1,
];

const NOISE_FREQ_TABLE: [isize; 5] = [0, 84, 140, 180, 210];

#[derive(Clone, Copy, Deserialize, PartialEq, Serialize)]
enum SpuTransferMode {
    Stop,
    ManualWrite,
    DmaWrite,
    DmaRead,
}

impl Default for SpuTransferMode {
    fn default() -> SpuTransferMode {
        SpuTransferMode::Stop
    }
}

impl From<u16> for SpuTransferMode {
    fn from(value: u16) -> SpuTransferMode {
        use self::SpuTransferMode::*;

        match value & 0x3 {
            0 => Stop,
            1 => ManualWrite,
            2 => DmaWrite,
            3 => DmaRead,
            _ => unreachable!(),
        }
    }
}

#[derive(Default, Deserialize, Serialize)]
struct SpuControl {
    enable: bool,
    mute: bool,
    noise_clock: u16,
    reverb_enable: bool,
    irq9_enable: bool,
    transfer_mode: SpuTransferMode,
    external_reverb: bool,
    cd_reverb: bool,
    external_enable: bool,
    cd_enable: bool,
}

impl SpuControl {
    pub fn read(&self) -> u16 {
        let mut value = 0;

        value |= (self.enable as u16) << 15;
        value |= (self.mute as u16) << 14;
        value |= (self.noise_clock & 0x3f) << 8;
        value |= (self.reverb_enable as u16) << 7;
        value |= (self.irq9_enable as u16) << 6;
        value |= (self.transfer_mode as u16) << 4;
        value |= (self.external_reverb as u16) << 3;
        value |= (self.cd_reverb as u16) << 2;
        value |= (self.external_enable as u16) << 1;
        value |= self.cd_enable as u16;

        value
    }

    pub fn write(&mut self, value: u16) -> bool {
        self.enable = (value & 0x8000) != 0;
        self.mute = (value & 0x4000) != 0;
        self.noise_clock = (value & 0x3f00) >> 8;
        self.reverb_enable = (value & 0x80) != 0;
        self.irq9_enable = (value & 0x40) != 0;
        self.transfer_mode = SpuTransferMode::from((value & 0x30) >> 4);
        self.external_reverb = (value & 0x8) != 0;
        self.cd_reverb = (value & 0x4) != 0;
        self.external_enable = (value & 0x2) != 0;
        self.cd_enable = (value & 0x1) != 0;

        !self.irq9_enable
    }
}

#[derive(Default, Deserialize, Serialize)]
struct SpuDataTransfer {
    address: u32,
    current: u32,

    fifo: VecDeque<u16>,

    control: u16,
}

#[derive(Deserialize, Serialize)]
pub struct SpuRam {
    data: Box<[u16]>,

    irq_address: u32,
    irq: bool,
}

impl SpuRam {
    pub fn new(size: usize) -> SpuRam {
        SpuRam {
            data: vec![0; size].into_boxed_slice(),

            irq_address: 0,
            irq: false,
        }
    }

    pub fn irq(&mut self) -> bool {
        let irq = self.irq;
        self.irq = false;
        irq
    }

    pub fn memory_read16(&mut self, address: u32) -> u16 {
        let index = (address & 0x7fffe) as usize;

        if address >= 0x800 && address < 0x1000 {
            println!(
                "[SPU] [WARN] Read from voice1/3 buffer at 0x{:08x}",
                address
            );
        }

        if (address & 0x7fffe) == self.irq_address {
            self.irq = true;
        }

        self.data[index / 2]
    }

    pub fn memory_write16(&mut self, address: u32, value: u16) {
        let index = (address & 0x7fffe) as usize;

        if (address & 0x7ffff) == self.irq_address {
            self.irq = true;
        }

        if ((address + 1) & 0x7ffff) == self.irq_address {
            self.irq = true;
        }

        self.data[index / 2] = value;
    }
}

#[derive(Deserialize, Serialize)]
pub struct Spu {
    output_buffer: Vec<i16>,

    cd_left_buffer: VecDeque<i16>,
    cd_right_buffer: VecDeque<i16>,

    capture_index: u32,

    sound_ram: SpuRam,

    voice: [Voice; SPU_NR_VOICES],

    main_volume: Volume,
    reverb_volume: Volume,

    key_on: u32,
    key_off: u32,
    endx: u32,
    echo_on: u32,

    modulate_on: u32,

    noise_on: u32,
    noise_timer: isize,
    noise_level: i16,

    control: SpuControl,

    reverb: Reverb,

    data_transfer: SpuDataTransfer,

    irq_status: bool,

    writing_to_capture_buffer_half: bool,
    data_transfer_busy: bool,
    data_transfer_dma_read: bool,
    data_transfer_dma_write: bool,

    cd_volume: Volume,
    extern_volume: Volume,
    current_volume: Volume,
}

impl Spu {
    pub fn new() -> Spu {
        Spu {
            output_buffer: Vec::with_capacity(SPU_BUFFER_SIZE),

            cd_left_buffer: VecDeque::new(),
            cd_right_buffer: VecDeque::new(),

            capture_index: 0,

            sound_ram: SpuRam::new(SPU_RAM_SIZE / SPU_WORD_SIZE),

            voice: [Voice::new(); SPU_NR_VOICES],

            main_volume: Volume::default(),
            reverb_volume: Volume::default(),

            key_on: 0,
            key_off: 0,
            endx: 0,
            echo_on: 0,

            modulate_on: 0,

            noise_on: 0,
            noise_timer: 0,
            noise_level: 1,

            control: SpuControl::default(),

            reverb: Reverb::default(),

            data_transfer: SpuDataTransfer::default(),

            irq_status: false,

            writing_to_capture_buffer_half: false,
            data_transfer_busy: false,
            data_transfer_dma_read: false,
            data_transfer_dma_write: false,

            cd_volume: Volume::default(),
            extern_volume: Volume::default(),
            current_volume: Volume::default(),
        }
    }

    fn update_key_on(&mut self) {
        for i in 0..24 {
            if (self.key_on & (1 << i)) != 0 {
                self.voice[i].key_on();
            }
        }

        self.key_on = 0;
    }

    fn update_key_off(&mut self) {
        for i in 0..24 {
            if (self.key_off & (1 << i)) != 0 {
                self.voice[i].key_off();
            }
        }

        self.key_off = 0;
    }

    fn update_echo(&mut self) {
        for i in 0..24 {
            if (self.echo_on & (1 << i)) != 0 {
                self.voice[i].echo_on();
            }
        }
    }

    fn update_endx(&mut self) {
        self.endx = 0;

        for i in 0..24 {
            if self.voice[i].endx() {
                self.endx |= 1 << i;
            }
        }
    }

    fn update_noise(&mut self) {
        for i in 0..24 {
            self.voice[i].set_noise((self.noise_on & (1 << i)) != 0);
        }

        let noise_clock = (self.control.noise_clock & 0x3) as usize;

        let mut level = 0x8000 >> (self.control.noise_clock >> 2);
        level <<= 16;

        self.noise_timer += 0x10000;
        self.noise_timer += NOISE_FREQ_TABLE[noise_clock];

        if (self.noise_timer & 0xffff) >= NOISE_FREQ_TABLE[4] {
            self.noise_timer += 0x10000;
            self.noise_timer -= NOISE_FREQ_TABLE[noise_clock];
        }

        if self.noise_timer >= level {
            self.noise_timer %= level;

            let bit = NOISE_WAVE_TABLE[((self.noise_level >> 10) & 0x3f) as usize] as i16;
            self.noise_level = (self.noise_level << 1) | bit;
        }
    }

    pub fn tick(&mut self, intc: &mut Intc) {
        let mut left = 0.0;
        let mut right = 0.0;

        let mut cd_left = 0.0;
        let mut cd_right = 0.0;

        let mut reverb_in_left = 0.0;
        let mut reverb_in_right = 0.0;

        if !self.cd_left_buffer.is_empty() {
            cd_left = i16_to_f32(self.cd_left_buffer.pop_front().unwrap());
        }

        if !self.cd_right_buffer.is_empty() {
            cd_right = i16_to_f32(self.cd_right_buffer.pop_front().unwrap());
        }

        /* TODO: Maybe do this upon writing only? */
        self.update_key_on();
        self.update_key_off();
        self.update_endx();
        self.update_echo();
        self.update_noise();

        let mut modulator = 0;
        let noise_level = i16_to_f32(self.noise_level);

        for i in 0..self.voice.len() {
            let voice = &mut self.voice[i];
            let modulate = i != 0 && (self.modulate_on & (1 << i)) != 0;
            let noise = (self.noise_on & (1 << i)) != 0;

            if voice.disabled() {
                continue;
            }

            let (sample_left, sample_right) = voice.get_samples(noise, noise_level);

            left += sample_left;
            right += sample_right;

            if voice.reverb_enabled() {
                reverb_in_left += sample_left;
                reverb_in_right += sample_right;
            }

            voice.update(&mut self.sound_ram, modulate, modulator);

            modulator = voice.modulator;
        }

        left *= self.main_volume.l();
        right *= self.main_volume.r();

        if self.control.reverb_enable {
            left += self.reverb.output_l() * self.reverb_volume.l();
            right += self.reverb.output_r() * self.reverb_volume.r();
        }

        if self.control.cd_enable {
            left += cd_left * self.cd_volume.l();
            right += cd_right * self.cd_volume.r();
        }

        if self.control.cd_reverb {
            reverb_in_left += cd_left * self.cd_volume.l();
            reverb_in_right += cd_right * self.cd_volume.r();
        }

        left = clip(left, -1.0, 1.0);
        right = clip(right, -1.0, 1.0);

        reverb_in_left = clip(reverb_in_left, -1.0, 1.0);
        reverb_in_right = clip(reverb_in_right, -1.0, 1.0);

        if self.control.reverb_enable {
            self.reverb
                .calculate(&mut self.sound_ram, [reverb_in_left, reverb_in_right]);
        }

        self.sound_ram
            .memory_write16(0x000 + self.capture_index, f32_to_i16(cd_left) as u16);
        self.sound_ram
            .memory_write16(0x400 + self.capture_index, f32_to_i16(cd_right) as u16);

        /* Fake writes to capture buffer */
        self.sound_ram.memory_write16(0x800 + self.capture_index, 0); /* Voice 1 */
        self.sound_ram.memory_write16(0xc00 + self.capture_index, 0); /* Voice 3 */

        self.capture_index = (self.capture_index + 2) & 0x3ff;
        self.writing_to_capture_buffer_half = self.capture_index >= 0x200;

        if self.sound_ram.irq() && self.control.irq9_enable {
            intc.assert_irq(Interrupt::Spu);
            self.irq_status = true;
        }

        /* TODO: Maybe ringbuffer? */
        self.output_buffer.push(f32_to_i16(left));
        self.output_buffer.push(f32_to_i16(right));
    }

    pub fn drain_samples(&mut self) -> Vec<i16> {
        self.output_buffer.drain(..).collect()
    }

    fn read_status(&self) -> u16 {
        let mut value = 0;
        let control = self.control.read();

        value |= (self.writing_to_capture_buffer_half as u16) << 11;
        value |= (self.data_transfer_busy as u16) << 10;
        value |= (self.data_transfer_dma_read as u16) << 9;
        value |= (self.data_transfer_dma_write as u16) << 8;
        value |= (control & 0x20) << 2;
        value |= (self.irq_status as u16) << 6;
        value |= control & 0x3f;

        value
    }

    fn push_fifo(&mut self, value: u16) {
        if self.data_transfer.fifo.len() < SPU_FIFO_SIZE {
            self.data_transfer.fifo.push_back(value);
        }
    }

    pub fn read16(&mut self, address: u32) -> u16 {
        match address {
            0x1f801c00..=0x1f801d7f => {
                let (voice, offset) = Voice::from_address(address);
                self.voice[voice].read16(offset)
            }
            0x1f801d80 => self.main_volume.left as u16,
            0x1f801d82 => self.main_volume.right as u16,
            0x1f801d84 => self.reverb_volume.left as u16,
            0x1f801d86 => self.reverb_volume.right as u16,
            0x1f801d88 => {
                println!("[SPU] [WARN] Read from KON register");
                self.key_on as u16
            }
            0x1f801d8a => {
                println!("[SPU] [WARN] Read from KON register");
                (self.key_on >> 16) as u16
            }
            0x1f801d8c => {
                println!("[SPU] [WARN] Read from KOFF register");
                self.key_off as u16
            }
            0x1f801d8e => {
                println!("[SPU] [WARN] Read from KOFF register");
                (self.key_off >> 16) as u16
            }
            0x1f801d90 => self.modulate_on as u16,
            0x1f801d92 => (self.modulate_on >> 16) as u16,
            0x1f801d94 => self.noise_on as u16,
            0x1f801d96 => (self.noise_on >> 16) as u16,
            0x1f801d98 => self.echo_on as u16,
            0x1f801d9a => (self.echo_on >> 16) as u16,
            0x1f801d9c => self.endx as u16,
            0x1f801d9e => (self.endx >> 16) as u16,
            0x1f801da2 => self.reverb.get_base(),
            0x1f801da6 => (self.data_transfer.address / 8) as u16,
            0x1f801da8 => {
                println!("[SPU] [WARN] Read from data transfer FIFO");
                0
            }
            0x1f801daa => self.control.read(),
            0x1f801dac => self.data_transfer.control,
            0x1f801dae => self.read_status(),
            0x1f801db0 => self.cd_volume.left as u16,
            0x1f801db2 => self.cd_volume.right as u16,
            0x1f801db4 => self.extern_volume.left as u16,
            0x1f801db6 => self.extern_volume.right as u16,
            0x1f801db8 => self.current_volume.left as u16,
            0x1f801dba => self.current_volume.right as u16,
            0x1f801dc0..=0x1f801dff => self.reverb.read16(address),
            0x1f801e00..=0x1f801fff => 0xffff,
            _ => panic!(
                "[SPU] [ERROR] Read from unimplemented register: 0x{:08x}",
                address
            ),
        }
    }

    pub fn read32(&mut self, address: u32) -> u32 {
        ((self.read16(address + 1) as u32) << 16) | self.read16(address) as u32
    }

    pub fn write16(&mut self, address: u32, value: u16) {
        match address {
            0x1f801c00..=0x1f801d7f => {
                let (voice, offset) = Voice::from_address(address);
                self.voice[voice].write16(offset, value)
            }
            0x1f801d80 => self.main_volume.left = value as i16,
            0x1f801d82 => self.main_volume.right = value as i16,
            0x1f801d84 => self.reverb_volume.left = value as i16,
            0x1f801d86 => self.reverb_volume.right = value as i16,
            0x1f801d88 => {
                self.key_on &= 0xffff0000;
                self.key_on |= value as u32;
            }
            0x1f801d8a => {
                self.key_on &= 0xffff;
                self.key_on |= (value as u32) << 16;
            }
            0x1f801d8c => {
                self.key_off &= 0xffff0000;
                self.key_off |= value as u32;
            }
            0x1f801d8e => {
                self.key_off &= 0xffff;
                self.key_off |= (value as u32) << 16;
            }
            0x1f801d90 => {
                self.modulate_on &= 0xffff0000;
                self.modulate_on |= value as u32;
            }
            0x1f801d92 => {
                self.modulate_on &= 0xffff;
                self.modulate_on |= (value as u32) << 16;
            }
            0x1f801d94 => {
                self.noise_on &= 0xffff0000;
                self.noise_on |= value as u32;
            }
            0x1f801d96 => {
                self.noise_on &= 0xffff;
                self.noise_on |= (value as u32) << 16;
            }
            0x1f801d98 => {
                self.echo_on &= 0xffff0000;
                self.echo_on |= value as u32;
            }
            0x1f801d9a => {
                self.echo_on &= 0xffff;
                self.echo_on |= (value as u32) << 16;
            }
            0x1f801d9c => println!("[SPU] [WARN] Write to ENDX register"),
            0x1f801d9e => println!("[SPU] [WARN] Write to ENDX register"),
            0x1f801da2 => self.reverb.set_base(value),
            0x1f801da4 => {
                self.sound_ram.irq_address = (value as u32) * 8;
                //println!("[SPU] IRQ Address: 0x{:08x}", self.sound_ram.irq_address);
            }
            0x1f801da6 => {
                self.data_transfer.address = (value as u32) * 8;
                self.data_transfer.current = (value as u32) * 8;
            }
            0x1f801da8 => self.push_fifo(value),
            0x1f801daa => {
                if self.control.write(value as u16) {
                    self.irq_status = false;
                }

                if self.control.transfer_mode == SpuTransferMode::ManualWrite {
                    while !self.data_transfer.fifo.is_empty() {
                        let data = self.data_transfer.fifo.pop_front().unwrap();
                        let address = self.data_transfer.current;

                        self.sound_ram.memory_write16(address, data);

                        self.data_transfer.current += 2;
                        self.data_transfer.current &= 0x7ffff;
                    }
                }
            }
            0x1f801dac => self.data_transfer.control = value,
            0x1f801dae => println!("[SPU] [WARN] Write to SPUSTAT"),
            0x1f801db0 => self.cd_volume.left = value as i16,
            0x1f801db2 => self.cd_volume.right = value as i16,
            0x1f801db4 => self.extern_volume.left = value as i16,
            0x1f801db6 => self.extern_volume.right = value as i16,
            0x1f801db8 => self.current_volume.left = value as i16,
            0x1f801dba => self.current_volume.right = value as i16,
            0x1f801dc0..=0x1f801dff => self.reverb.write16(address, value),
            _ => panic!(
                "[SPU] [ERROR] Write to unimplemented register: 0x{:08x}",
                address
            ),
        };
    }

    pub fn cd_push(&mut self, left: i16, right: i16) {
        self.cd_left_buffer.push_back(left);
        self.cd_right_buffer.push_back(right);
    }

    pub fn cd_push_left(&mut self, sample: i16) {
        self.cd_left_buffer.push_back(sample);
    }

    pub fn cd_push_right(&mut self, sample: i16) {
        self.cd_right_buffer.push_back(sample);
    }

    pub fn dma_read(&mut self) -> u32 {
        let address = self.data_transfer.current;

        let lo = self.sound_ram.memory_read16(address) as u32;
        let hi = self.sound_ram.memory_read16(address + 2) as u32;

        self.data_transfer.current += 4;
        self.data_transfer.current &= 0x7ffff;

        (hi << 16) | lo
    }

    pub fn dma_write(&mut self, value: u32) {
        let address = self.data_transfer.current;

        self.sound_ram.memory_write16(address, value as u16);
        self.sound_ram
            .memory_write16(address + 2, (value >> 16) as u16);

        self.data_transfer.current += 4;
        self.data_transfer.current &= 0x7ffff;
    }
}
