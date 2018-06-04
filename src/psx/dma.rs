#[derive(Clone, Copy)]
pub enum SyncMode {
    Manual,
    Request,
    LinkedList,
}

#[derive(Clone, Copy)]
pub enum Step {
    Forward,
    Backward,
}

#[derive(Clone, Copy, Debug)]
pub enum Direction {
    ToRam,
    FromRam,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DmaPort {
    MDECIn,
    MDECOut,
    GPU,
    CDROM,
    SPU,
    PIO,
    OTC,
}

impl DmaPort {
    pub fn to(port: usize) -> DmaPort {
        use self::DmaPort::*;

        match port {
            0 => MDECIn,
            1 => MDECOut,
            2 => GPU,
            3 => CDROM,
            4 => SPU,
            5 => PIO,
            6 => OTC,
            _ => panic!("[ERROR] [DMA] Invalid port {}", port),
        }
    }

    pub fn from(port: DmaPort) -> usize {
        use self::DmaPort::*;

        match port {
            MDECIn => 0,
            MDECOut => 1,
            GPU => 2,
            CDROM => 3,
            SPU => 4,
            PIO => 5,
            OTC => 6,
        }
    }
}

#[derive(Clone, Copy)]
pub struct DmaChannel {
    base_address: u32,

    block_size: u16,
    block_amount: u16,

    trigger: bool,
    enable: bool,
    sync: SyncMode,
    step: Step,
    direction: Direction,
}

impl DmaChannel {
    pub fn new() -> DmaChannel {
        DmaChannel {
            base_address: 0,

            block_size: 0,
            block_amount: 0,

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

    pub fn block_size(&self) -> u16 {
        self.block_size
    }

    pub fn block_amount(&self) -> u16 {
        self.block_amount
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
        self.trigger = (value & 0x1000_0000) != 0;
        self.enable = (value & 0x0100_0000) != 0;
        self.sync = match (value & 0x600) >> 9 {
            0 => SyncMode::Manual,
            1 => SyncMode::Request,
            2 => SyncMode::LinkedList,
            3 => panic!("[DMA] [ERROR] Invalid SyncMode"),
            _ => unreachable!(),
        };
        self.step = match (value & 0x2) != 0 {
            true => Step::Backward,
            false => Step::Forward,
        };
        self.direction = match (value & 0x1) != 0 {
            true => Direction::FromRam,
            false => Direction::ToRam,
        };
    }
}

pub struct Dma {
    channels: [DmaChannel; 7],
    control: u32,
    interrupt: u32,
}

impl Dma {
    pub fn new() -> Dma {
        Dma {
            channels: [DmaChannel::new(); 7],
            control: 0x07654321,
            interrupt: 0,
        }
    }

    pub fn channel(&self, port: DmaPort) -> DmaChannel {
        let channel = DmaPort::from(port);
        self.channels[channel]
    }

    pub fn channel_mut(&mut self, port: DmaPort) -> &mut DmaChannel {
        let channel = DmaPort::from(port);
        &mut self.channels[channel]
    }

    pub fn read(&self, address: u32) -> u32 {
        let section = (address as usize & 0x70) >> 4;
        let register = address & 0x0f;

        match section {
            0...5 => {
                let port = DmaPort::to(section);
                let channel = self.channel(port);

                match register {
                    0 => channel.base_address,
                    4 => channel.block_control_read(),
                    8 => channel.channel_control_read(),
                    _ => panic!("[ERROR] [DMA] Unknown DMA read 0x{:08x}", address),
                }  
            },
            6 => {
                let channel = self.channel(DmaPort::OTC);

                match register {
                    0 => channel.base_address,
                    4 => channel.block_control_read(),
                    8 => channel.channel_control_read() | 0x0000_0002,
                    _ => panic!("[ERROR] [DMA] Unknown DMA read 0x{:08x}", address),
                }
            },
            7 => match register {
                0 => self.control,
                4 => self.interrupt,
                _ => panic!("[ERROR] [DMA] Unknown DMA read 0x{:08x}", address),
            },
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, address: u32, value: u32) -> Option<DmaPort> {
        let section = (address as usize & 0x70) >> 4;
        let register = address & 0x0f;

        let mut active: Option<DmaPort> = None;

        match section {
            0...5 => {
                let port = DmaPort::to(section);
                let channel = self.channel_mut(port);

                match register {
                    0 => channel.base_address = value,
                    4 => channel.block_control_write(value),
                    8 => channel.channel_control_write(value),
                    _ => panic!("[ERROR] [DMA] Unknown DMA write 0x{:08x}", address),
                };

                if channel.active() {
                    active = Some(port)
                }
            },
            6 => {
                let port = DmaPort::to(section);
                let channel = self.channel_mut(port);

                match register {
                    0 => channel.base_address = value,
                    4 => channel.block_control_write(value),
                    8 => channel.channel_control_write((value & 0x5100_0000) | 0x0000_0002),
                    _ => panic!("[ERROR] [DMA] Unknown DMA write 0x{:08x}", address),
                };

                if channel.active() {
                    active = Some(port);
                }
            },
            7 => match register {
                0 => self.control = value,
                4 => self.interrupt = value,
                _ => panic!("[ERROR] [DMA] Unknown DMA write 0x{:08x}", address),
            },
            _ => unreachable!(),
        };

        active
    }
}