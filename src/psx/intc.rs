use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq)]
pub enum Interrupt {
    Vblank,
    Gpu,
    Cdrom,
    Dma,
    Tmr0,
    Tmr1,
    Tmr2,
    Controller,
    Sio,
    Spu,
    Pio,
}

impl Interrupt {
    pub fn to_u32(interrupt: &Interrupt) -> u32 {
        use self::Interrupt::*;

        match interrupt {
            Vblank => 0x1,
            Gpu => 0x2,
            Cdrom => 0x4,
            Dma => 0x8,
            Tmr0 => 0x10,
            Tmr1 => 0x20,
            Tmr2 => 0x40,
            Controller => 0x80,
            Sio => 0x100,
            Spu => 0x200,
            Pio => 0x400,
        }
    }
}

#[derive(Deserialize, Serialize)]
struct InterruptRegister {
    pio: bool,
    spu: bool,
    sio: bool,
    controller: bool,
    tmr2: bool,
    tmr1: bool,
    tmr0: bool,
    dma: bool,
    cdrom: bool,
    gpu: bool,
    vblank: bool,
}

impl InterruptRegister {
    pub fn new() -> InterruptRegister {
        InterruptRegister {
            pio: false,
            spu: false,
            sio: false,
            controller: false,
            tmr2: false,
            tmr1: false,
            tmr0: false,
            dma: false,
            cdrom: false,
            gpu: false,
            vblank: false,
        }
    }

    pub fn read(&self) -> u32 {
        let mut value = 0;

        value |= (self.pio as u32) << 10;
        value |= (self.spu as u32) << 9;
        value |= (self.sio as u32) << 8;
        value |= (self.controller as u32) << 7;
        value |= (self.tmr2 as u32) << 6;
        value |= (self.tmr1 as u32) << 5;
        value |= (self.tmr0 as u32) << 4;
        value |= (self.dma as u32) << 3;
        value |= (self.cdrom as u32) << 2;
        value |= (self.gpu as u32) << 1;
        value |= (self.vblank as u32) << 0;

        value
    }

    pub fn write(&mut self, value: u32) {
        self.pio = (value & 0x400) != 0;
        self.spu = (value & 0x200) != 0;
        self.sio = (value & 0x100) != 0;
        self.controller = (value & 0x80) != 0;
        self.tmr2 = (value & 0x40) != 0;
        self.tmr1 = (value & 0x20) != 0;
        self.tmr0 = (value & 0x10) != 0;
        self.dma = (value & 0x8) != 0;
        self.cdrom = (value & 0x4) != 0;
        self.gpu = (value & 0x2) != 0;
        self.vblank = (value & 0x1) != 0;
    }
}

#[derive(Deserialize, Serialize)]
pub struct Intc {
    status: InterruptRegister,
    mask: InterruptRegister,

    pending: bool,
}

impl Intc {
    pub fn new() -> Intc {
        Intc {
            status: InterruptRegister::new(),
            mask: InterruptRegister::new(),

            pending: false,
        }
    }

    pub fn pending(&self) -> bool {
        self.pending
    }

    fn update_pending(&mut self) {
        let status = self.status.read();
        let mask = self.mask.read();

        self.pending = (status & mask) != 0;
    }

    pub fn assert_irq(&mut self, interrupt: Interrupt) {
        let status = self.status.read();
        self.status.write(status | Interrupt::to_u32(&interrupt));

        self.update_pending();
    }

    pub fn read_status(&self) -> u32 {
        self.status.read()
    }

    pub fn acknowledge_irq(&mut self, value: u32) {
        let status = self.status.read();
        self.status.write(status & value);

        self.update_pending();
    }

    pub fn read_mask(&self) -> u32 {
        self.mask.read()
    }

    pub fn write_mask(&mut self, value: u32) {
        self.mask.write(value);

        self.update_pending();
    }
}
