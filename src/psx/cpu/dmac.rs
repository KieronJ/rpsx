use byteorder::{ByteOrder, LittleEndian};
use serde::{Deserialize, Serialize};

use super::Bus;
use super::super::intc::{Intc, Interrupt};

#[derive(Clone, Copy, Deserialize, Serialize)]
pub enum SyncMode {
    Manual,
    Request,
    LinkedList,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
pub enum Step {
    Forward,
    Backward,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum Direction {
    ToRam,
    FromRam,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum DmacPort {
    MDECIn,
    MDECOut,
    GPU,
    CDROM,
    SPU,
    PIO,
    OTC,
    Control,
}

impl DmacPort {
    pub fn to(port: usize) -> DmacPort {
        use self::DmacPort::*;

        match port {
            0 => MDECIn,
            1 => MDECOut,
            2 => GPU,
            3 => CDROM,
            4 => SPU,
            5 => PIO,
            6 => OTC,
            7 => Control,
            _ => panic!("[ERROR] [DMAC] Invalid port {}", port),
        }
    }

    pub fn from(port: DmacPort) -> usize {
        use self::DmacPort::*;

        match port {
            MDECIn => 0,
            MDECOut => 1,
            GPU => 2,
            CDROM => 3,
            SPU => 4,
            PIO => 5,
            OTC => 6,
            Control => 7,
        }
    }
}

#[derive(Clone, Copy, Deserialize, Serialize)]
pub struct DmacChannel {
    base_address: u32,

    block_size: u16,
    block_amount: u16,

    chopping_enabled: bool,

    trigger: bool,
    enable: bool,
    sync: SyncMode,
    step: Step,
    direction: Direction,
}

impl DmacChannel {
    pub fn new() -> DmacChannel {
        DmacChannel {
            base_address: 0,

            block_size: 0,
            block_amount: 0,

            chopping_enabled: false,

            trigger: false,
            enable: false,
            sync: SyncMode::Manual,
            step: Step::Forward,
            direction: Direction::ToRam,
        }
    }

    pub fn base_address(&self) -> u32 {
        self.base_address & 0xfffffc
    }

    pub fn block_size(&self) -> usize {
        if self.block_size == 0 {
            return 0x10000;
        }

        self.block_size as usize
    }

    pub fn sync(&self) -> SyncMode {
        self.sync
    }

    pub fn step(&self) -> Step {
        self.step
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    pub fn active(&self) -> bool {
        let trigger = match self.sync {
            SyncMode::Manual => self.trigger,
            _ => true,
        };

        self.enable & trigger
    }

    pub fn finish(&mut self) {
        self.trigger = false;
        self.enable = false;
    }

    pub fn block_control_read(&self) -> u32 {
        ((self.block_amount as u32) << 16) | (self.block_size as u32)
    }

    pub fn block_control_write(&mut self, value: u32) {
        self.block_size = value as u16;
        self.block_amount = (value >> 16) as u16;
    }

    pub fn channel_control_read(&self) -> u32 {
        let mut value = 0;

        value |= (self.trigger as u32) << 28;
        value |= (self.enable as u32) << 24;
        value |= match self.sync {
            SyncMode::Manual => 0x000,
            SyncMode::Request => 0x200,
            SyncMode::LinkedList => 0x400,
        };
        value |= match self.step {
            Step::Forward => 0x0,
            Step::Backward => 0x2,
        };
        value |= match self.direction {
            Direction::ToRam => 0x0,
            Direction::FromRam => 0x1,
        };

        value
    }

    pub fn channel_control_write(&mut self, value: u32) {
        let old_enable = self.enable;

        self.trigger = (value & 0x1000_0000) != 0;
        self.enable = (value & 0x0100_0000) != 0;
        self.sync = match (value & 0x600) >> 9 {
            0 => SyncMode::Manual,
            1 => SyncMode::Request,
            2 => SyncMode::LinkedList,
            3 => panic!("[DMAC] [ERROR] Invalid SyncMode"),
            _ => unreachable!(),
        };

        self.chopping_enabled = (value & 0x100) != 0;

        self.step = match (value & 0x2) != 0 {
            true => Step::Backward,
            false => Step::Forward,
        };
        self.direction = match (value & 0x1) != 0 {
            true => Direction::FromRam,
            false => Direction::ToRam,
        };

        if old_enable && !self.enable {
            panic!("disabled active transfer");
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct Dmac {
    channels: [DmacChannel; 7],
    control: u32,
    interrupt: u32,

    gap_ticks: isize,
    gap_started: bool,

    active_port: Option<DmacPort>,
    active_address: u32,
    active_remaining: usize,
    active_count: usize,
}

impl Dmac {
    pub fn new() -> Dmac {
        Dmac {
            channels: [DmacChannel::new(); 7],
            control: 0x07654321,
            interrupt: 0,

            gap_ticks: 0,
            gap_started: false,

            active_port: None,
            active_address: 0,
            active_remaining: 0,
            active_count: 0,
        }
    }

    fn tick_manual(&mut self, port: DmacPort, bus: &mut Bus) {
        let channel = self.channel(port);

        let step = channel.step();
        let direction = channel.direction();

        match direction {
            Direction::ToRam => {
                match port {
                    DmacPort::CDROM => {
                        let data = bus.cdrom().data_dma();

                        LittleEndian::write_u32(
                            &mut bus.ram()[self.active_address as usize..],
                            data,
                        );

                        self.active_address = match step {
                            Step::Forward => self.active_address.wrapping_add(4),
                            Step::Backward => self.active_address.wrapping_sub(4),
                        } & 0x1f_fffc;

                        self.active_remaining -= 1;
                    }
                    DmacPort::OTC => {
                        let value = match self.active_remaining {
                            1 => 0xff_ffff,
                            _ => self.active_address.wrapping_sub(4) & 0x1f_fffc,
                        };

                        LittleEndian::write_u32(
                            &mut bus.ram()[self.active_address as usize..],
                            value,
                        );

                        self.active_address = value & 0x1f_fffc;
                        self.active_remaining -= 1;
                    }
                    _ => panic!("[DMAC] [ERROR] Unsupported DMA Port {:?} for Manual", port),
                };
            }
            Direction::FromRam => {
                match port {
                    DmacPort::GPU => {
                        let data =
                            LittleEndian::read_u32(&bus.ram()[self.active_address as usize..]);

                        bus.gpu_mut().gp0_write(data);

                        self.active_address = match step {
                            Step::Forward => self.active_address.wrapping_add(4),
                            Step::Backward => self.active_address.wrapping_sub(4),
                        } & 0x1f_fffc;

                        self.active_remaining -= 1;
                    }
                    _ => panic!("[DMAC] [ERROR] Unsupported DMA Port {:?} for Manual", port),
                };
            }
        };

        if self.active_remaining == 0 {
            self.active_count += channel.block_size();
            self.active_port = None;
            self.channel_mut(port).finish();
            self.finish_set_interrupt(bus.intc(), port);
        }
    }

    fn tick_request(&mut self, port: DmacPort, bus: &mut Bus) {
        let channel = self.channel(port);

        let step = channel.step();
        let direction = channel.direction();

        match direction {
            Direction::ToRam => {
                match port {
                    DmacPort::MDECOut => {
                        let data = bus.mdec().read_data();

                        LittleEndian::write_u32(
                            &mut bus.ram()[self.active_address as usize..],
                            data,
                        );

                        self.active_address = match step {
                            Step::Forward => self.active_address.wrapping_add(4),
                            Step::Backward => self.active_address.wrapping_sub(4),
                        } & 0x1f_fffc;

                        self.active_remaining -= 1;
                    }
                    DmacPort::GPU => {
                        let data = bus.gpu_mut().gpuread();

                        LittleEndian::write_u32(
                            &mut bus.ram()[self.active_address as usize..],
                            data,
                        );

                        self.active_address = match step {
                            Step::Forward => self.active_address.wrapping_add(4),
                            Step::Backward => self.active_address.wrapping_sub(4),
                        } & 0x1f_fffc;

                        self.active_remaining -= 1;
                    }
                    DmacPort::SPU => {
                        let data = bus.spu().dma_read();

                        LittleEndian::write_u32(
                            &mut bus.ram()[self.active_address as usize..],
                            data,
                        );

                        self.active_address = match step {
                            Step::Forward => self.active_address.wrapping_add(4),
                            Step::Backward => self.active_address.wrapping_sub(4),
                        } & 0x1f_fffc;

                        self.active_remaining -= 1;
                    }
                    _ => panic!("[DMAC] [ERROR] Unsupported DMA Port {:?} for Request", port),
                };
            }
            Direction::FromRam => {
                match port {
                    DmacPort::MDECIn => {
                        let data =
                            LittleEndian::read_u32(&bus.ram()[self.active_address as usize..]);

                        bus.mdec().write_command(data);

                        self.active_address = match step {
                            Step::Forward => self.active_address.wrapping_add(4),
                            Step::Backward => self.active_address.wrapping_sub(4),
                        } & 0x1f_fffc;

                        self.active_remaining -= 1;
                    }
                    DmacPort::GPU => {
                        let data =
                            LittleEndian::read_u32(&bus.ram()[self.active_address as usize..]);

                        bus.gpu_mut().gp0_write(data);

                        self.active_address = match step {
                            Step::Forward => self.active_address.wrapping_add(4),
                            Step::Backward => self.active_address.wrapping_sub(4),
                        } & 0x1f_fffc;

                        self.active_remaining -= 1;
                    }
                    DmacPort::SPU => {
                        let data =
                            LittleEndian::read_u32(&bus.ram()[self.active_address as usize..]);

                        bus.spu().dma_write(data);

                        self.active_address = match step {
                            Step::Forward => self.active_address.wrapping_add(4),
                            Step::Backward => self.active_address.wrapping_sub(4),
                        } & 0x1f_fffc;

                        self.active_remaining -= 1;
                    }
                    _ => panic!("[DMAC] [ERROR] Unsupported DMA Port {:?} for Request", port),
                };
            }
        };

        if self.active_remaining == 0 {
            self.active_count += channel.block_size();

            let channel = self.channel_mut(port);
            channel.block_amount -= 1;
            channel.base_address += channel.block_size() as u32;

            let chopping = channel.chopping_enabled;

            if channel.block_amount == 0 {
                channel.finish();
                self.active_port = None;
                self.finish_set_interrupt(bus.intc(), port);
            } else {
                self.active_remaining = channel.block_size();
                self.gap_ticks += 1;
                self.gap_started = chopping;
            }
        }
    }

    fn tick_linked_list(&mut self, port: DmacPort, bus: &mut Bus) {
        let channel = self.channel(port);
        let direction = channel.direction();

        if direction != Direction::FromRam {
            panic!("[DMAC] [ERROR] Unsupported direction {:?} for linked list transfer", direction);
        }

        if port != DmacPort::GPU {
            panic!("[DMAC] [ERROR] Unsupported {:?} for linked list transfer", port);
        }

        if self.gap_ticks > 0 {
            self.gap_ticks += 1;
            return;
        }

        let header = LittleEndian::read_u32(&bus.ram()[self.active_address as usize..]);
        let payload_length = header >> 24;

        for _ in 0..payload_length {
            self.active_address = (self.active_address + 4) & 0x1f_fffc;

            let command = LittleEndian::read_u32(&bus.ram()[self.active_address as usize..]);
            bus.gpu_mut().gp0_write(command);
        }

        self.active_count += payload_length as usize;
        self.active_address = header & 0x1f_fffc;
        self.channel_mut(port).base_address = header & 0x1f_fffc;

        let chopping = channel.chopping_enabled;

        if (header & 0x80_0000) != 0 {
            self.channel_mut(port).finish();

            self.active_port = None;
            self.finish_set_interrupt(bus.intc(), port);
        } else {
            self.gap_started = chopping;
            self.gap_ticks += 1;
        }
    }

    pub fn tick(&mut self, bus: &mut Bus) -> usize {
        let mut count = 0;

        if let Some(p) = self.active_port {
            let channel = self.channel(p);
            let sync = channel.sync();

            if self.dma_enabled(p) {
                match sync {
                    SyncMode::Manual => self.tick_manual(p, bus),
                    SyncMode::Request => self.tick_request(p, bus),
                    SyncMode::LinkedList => self.tick_linked_list(p, bus),
                };
            } else {
                self.active_port = None;
            }

            count = self.active_count;
            self.active_count = 0;
        }

        count
    }

    pub fn active(&self) -> bool {
        self.active_port.is_some()
    }

    pub fn in_gap(&self) -> bool {
        self.gap_ticks > 0
    }

    pub fn gap_started(&mut self) -> bool {
        let gap_started = self.gap_started && self.gap_ticks > 32;

        if gap_started {
            self.gap_started = false;
        }

        gap_started
    }

    pub fn chopping_enabled(&self) -> bool {
        let channel = DmacPort::from(self.active_port.unwrap());
        return self.channels[channel].chopping_enabled;
    }

    pub fn tick_gap(&mut self, ticks: usize) {
        self.gap_ticks -= ticks as isize;
    }

    pub fn channel(&self, port: DmacPort) -> DmacChannel {
        let channel = DmacPort::from(port);
        self.channels[channel]
    }

    pub fn channel_mut(&mut self, port: DmacPort) -> &mut DmacChannel {
        let channel = DmacPort::from(port);
        &mut self.channels[channel]
    }

    pub fn dma_enabled(&self, port: DmacPort) -> bool {
        let p = DmacPort::from(port);

        (self.control & (1 << ((p << 2) + 3))) != 0
    }

    pub fn finish_set_interrupt(&mut self, intc: &mut Intc, port: DmacPort) {
        let bit = DmacPort::from(port);

        let mask = 1 << (16 + bit);
        let status = 1 << (24 + bit);

        if self.interrupt & mask != 0 {
            self.interrupt |= status;
        }

        self.update_master_flag(intc);
    }

    fn update_master_flag(&mut self, intc: &mut Intc) {
        let prev_master = (self.interrupt & 0x8000_0000) != 0;

        let force = (self.interrupt & (1 << 15)) != 0;
        let master_enable = (self.interrupt & (1 << 23)) != 0;
        let flag = (self.interrupt & 0x7f00_0000) >> 24;
        let enable = (self.interrupt & 0x007f_0000) >> 16;

        let interrupt_enable = (flag & enable) != 0;

        self.interrupt &= !0x8000_0000;

        if force | (master_enable & interrupt_enable) {
            self.interrupt |= 0x8000_0000;

            if !prev_master {
                intc.assert_irq(Interrupt::Dma);
            }
        }
    }

    pub fn read(&self, address: u32) -> u32 {
        let section = (address as usize & 0x70) >> 4;
        let register = address & 0x0f;

        match section {
            0..=5 => {
                let port = DmacPort::to(section);
                let channel = self.channel(port);

                match register {
                    0 => channel.base_address,
                    4 => channel.block_control_read(),
                    8 => channel.channel_control_read(),
                    _ => panic!("[ERROR] [DMAC] Unknown DMA read 0x{:08x}", address),
                }
            }
            6 => {
                let channel = self.channel(DmacPort::OTC);

                match register {
                    0 => channel.base_address,
                    4 => channel.block_control_read(),
                    8 => channel.channel_control_read() | 0x0000_0002,
                    _ => panic!("[ERROR] [DMAC] Unknown DMA read 0x{:08x}", address),
                }
            }
            7 => match register {
                0 => self.control,
                4 => self.interrupt,
                6 => self.interrupt >> 16,
                _ => panic!("[ERROR] [DMAC] Unknown DMA read 0x{:08x}", address),
            },
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, intc: &mut Intc, address: u32, value: u32) {
        let section = (address as usize & 0x70) >> 4;
        let register = address & 0x0f;

        let port = DmacPort::to(section);

        match section {
            0..=5 => {
                let channel = self.channel_mut(port);

                match register {
                    0 => channel.base_address = value & 0xfffffc,
                    4 => channel.block_control_write(value),
                    8 => channel.channel_control_write(value),
                    _ => panic!("[ERROR] [DMAC] Unknown DMA write 0x{:08x}", address),
                };
            }
            6 => {
                let channel = self.channel_mut(port);

                match register {
                    0 => channel.base_address = value & 0xfffffc,
                    4 => channel.block_control_write(value),
                    8 => channel.channel_control_write((value & 0x5100_0000) | 0x0000_0002),
                    _ => panic!("[ERROR] [DMAC] Unknown DMA write 0x{:08x}", address),
                };
            }
            7 => match register {
                0 => self.control = value,
                4 => {
                    self.interrupt &= 0xff00_0000;
                    self.interrupt &= !(value & 0x7f00_0000);
                    self.interrupt |= value & 0xff_803f;
                    self.update_master_flag(intc);
                }
                6 => {
                    self.interrupt &= 0xff00_0000;
                    self.interrupt &= !((value << 16) & 0x7f00_0000);
                    self.interrupt |= (value << 16) & 0xff_0000;
                    self.update_master_flag(intc);
                }
                _ => panic!("[ERROR] [DMAC] Unknown DMA write 0x{:08x}", address),
            },
            _ => unreachable!(),
        };

        if section == 7 {
            return;
        }

        let channel = self.channel(port);

        if channel.active() {
            self.active_port = Some(port);

            match channel.sync() {
                SyncMode::Manual => {
                    self.active_address = channel.base_address() & 0x1f_fffc;
                    self.active_remaining = channel.block_size();
                }
                SyncMode::Request => {
                    self.active_address = channel.base_address() & 0x1f_fffc;
                    self.active_remaining = channel.block_size();
                }
                SyncMode::LinkedList => {
                    self.active_address = channel.base_address() & 0x1f_fffc;
                    self.active_remaining = 1;
                }
            }

            self.active_count = 0;

            if self.active_remaining == 0 {
                self.active_port = None;
            }
        }
    }
}
