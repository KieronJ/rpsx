use super::bus::Bus;

use serde::{Deserialize, Serialize};

const DEVICE_COUNT: usize = 5;
const DEVICE_GRANULARITY: [u64; DEVICE_COUNT] = [7, 8448, 8448, 11, 11];

const DMAC_GRANULARITY: u64 = 11;

#[derive(Clone, Copy)]
pub enum Device {
    Gpu,
    Cdrom,
    Spu,
    Timers,
    Sio0,
}

impl Device {
    pub fn from(value: usize) -> Device {
        match value {
            0 => Device::Gpu,
            1 => Device::Cdrom,
            2 => Device::Spu,
            3 => Device::Timers,
            4 => Device::Sio0,
            _ => unreachable!(),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct Timekeeper {
    now: u64,
    last_sync: u64,

    devices: [u64; DEVICE_COUNT],
    dmac: u64,
}

impl Timekeeper {
    pub fn new() -> Timekeeper {
        Timekeeper {
            now: 0,
            last_sync: 0,

            devices: [0; DEVICE_COUNT],
            dmac: 0,
        }
    }

    pub fn reset(&mut self) {
        self.now = 0;
        self.last_sync = 0;

        self.devices = [0; DEVICE_COUNT];
        self.dmac = 0;
    }

    pub fn tick(&mut self, cycles: u64) {
        self.now += cycles * 11;
    }

    pub fn sync_all(&mut self, bus: &mut Bus) {
        self.last_sync = self.now;

        for i in 0..DEVICE_COUNT {
            self.sync_device(bus, Device::from(i));
        }
    }

    pub fn sync_device(&mut self, bus: &mut Bus, device: Device) {
        let elapsed = self.now - self.devices[device as usize];
        let cycles = elapsed / DEVICE_GRANULARITY[device as usize];

        self.devices[device as usize] += cycles * DEVICE_GRANULARITY[device as usize];
        bus.tick_device_by_id(device, cycles as usize);
    }

    pub fn sync_dmac(&mut self) -> usize {
        let elapsed = self.now - self.dmac;
        let cycles = elapsed / DMAC_GRANULARITY;

        self.dmac += cycles * DMAC_GRANULARITY;
        cycles as usize
    }

    pub fn elapsed(&self) -> u64 {
        (self.now - self.last_sync) / 11
    }
}