use byteorder::{LittleEndian, ByteOrder};

use super::cdrom::Cdrom;
use super::dma::{Direction, Dma, DmaPort, Step, SyncMode};
use super::gpu::Gpu;
use super::timer::Timer;
use util;

pub enum BusWidth {
    BYTE,
    HALF,
    WORD,
}

pub struct Bus {
    bios: Box<[u8]>,
    ram: Box<[u8]>,

    cdrom: Cdrom,
    dma: Dma,
    gpu: Gpu,
    timer0: Timer,
    timer1: Timer,
    timer2: Timer,
}

impl Bus {
    pub fn new(bios: &str) -> Bus
    {
        Bus {
            bios: util::read_file_to_box(bios),
            ram: vec![0; 0x200000].into_boxed_slice(),

            cdrom: Cdrom::new(),
            dma: Dma::new(),
            gpu: Gpu::new(),
            timer0: Timer::new(0),
            timer1: Timer::new(1),
            timer2: Timer::new(2),
        }
    }

    pub fn cdrom(&mut self) -> &mut Cdrom {
        &mut self.cdrom
    }

    pub fn gpu(&mut self) -> &mut Gpu {
        &mut self.gpu
    }

    pub fn timer0(&mut self) -> &mut Timer {
        &mut self.timer0
    }

    pub fn timer1(&mut self) -> &mut Timer {
        &mut self.timer1
    }

    pub fn timer2(&mut self) -> &mut Timer {
        &mut self.timer2
    }

    fn fetch(&mut self, address: u32) -> u32 {
        match address {
            0x0000_0000...0x001f_ffff => LittleEndian::read_u32(&self.ram[address as usize..]),
            0x1f00_0000...0x1f07_ffff => { 0xffff_ffff }, //println!("[MMU] [INFO] Load from EXPENSION_1 region address: 0x{:08x}", address); 0xffff_ffff },
            0x1f80_1040...0x1f80_104f => { 0xffff_ffff },
            0x1f80_1080...0x1f80_10ff => self.dma.read(address),
            0x1f80_1100 => self.timer0.read_value(),
            0x1f80_1104 => self.timer0.read_mode(),
            0x1f80_1108 => self.timer0.read_target(),
            0x1f80_1110 => self.timer1.read_value(),
            0x1f80_1114 => self.timer1.read_mode(),
            0x1f80_1118 => self.timer1.read_target(),
            0x1f80_1120 => self.timer2.read_value(),
            0x1f80_1124 => self.timer2.read_mode(),
            0x1f80_1128 => self.timer2.read_target(),
            0x1f80_1800 => self.cdrom.read_status_register(),
            0x1f80_1801 => self.cdrom.read_register1(),
            0x1f80_1802 => self.cdrom.read_register2(),
            0x1f80_1803 => self.cdrom.read_register3(),
            0x1f80_1810 => self.gpu.gpuread(),
            0x1f80_1814 => self.gpu.gpustat(),
            0x1f80_1c00...0x1f80_1fff => 0, //{ println!("[MMU] [INFO] Load from SPU region address: 0x{:08x}", address); 0 },
            0x1fc0_0000...0x1fc7_ffff => LittleEndian::read_u32(&self.bios[address as usize - 0x1fc0_0000..]),
            _ => panic!("[BUS] [ERROR] Load from unrecognised address 0x{:08x}", address),
        }
    }

    pub fn load(&mut self, width: BusWidth, address: u32) -> u32 {
        use self::BusWidth::*;

        let mask = match width {
            BYTE => 0x0000_00ff,
            HALF => 0x0000_ffff,
            WORD => 0xffff_ffff,
        };

        self.fetch(address) & mask
    }

    pub fn store(&mut self, width: BusWidth, address: u32, value: u32) {
        use self::BusWidth::*;

        match address {
            0x0000_0000...0x001f_ffff => {
                let slice = &mut self.ram[address as usize..];

                match width {
                    BYTE => {
                        slice[0] = value as u8;
                    },

                    HALF => {
                        LittleEndian::write_u16(slice, value as u16);
                    },

                    WORD => {
                        LittleEndian::write_u32(slice, value);
                    },
                }
            },
            0x1f80_1000...0x1f80_1023 => (),//println!("[BUS] [INFO] Store to MEM_CTRL region address: 0x{:08x}", address),
            0x1f80_1040...0x1f80_104f => (),//println!("[BUS] [INFO] Store to JOYPAD region address: 0x{:08x}", address),
            0x1f80_1060 => (),//println!("[BUS] [INFO] Store to MEM_CTRL region address: 0x{:08x}", address),
            0x1f80_1080...0x1f80_10ff => {
                if let Some(port) = self.dma.write(address, value) {
                    //println!("[DMA] [INFO] Executing DMA\nPort: {:?}", port);
                    self.execute_dma(port);
                }
            },
            0x1f80_1100 => self.timer0.write_value(value),
            0x1f80_1104 => self.timer0.write_mode(value),
            0x1f80_1108 => self.timer0.write_target(value),
            0x1f80_1110 => self.timer1.write_value(value),
            0x1f80_1114 => self.timer1.write_mode(value),
            0x1f80_1118 => self.timer1.write_target(value),
            0x1f80_1120 => self.timer2.write_value(value),
            0x1f80_1124 => self.timer2.write_mode(value),
            0x1f80_1128 => self.timer2.write_target(value),
            0x1f80_1800 => self.cdrom.write_index_register(value),
            0x1f80_1801 => self.cdrom.write_register1(value),
            0x1f80_1802 => self.cdrom.write_register2(value),
            0x1f80_1803 => self.cdrom.write_register3(value),
            0x1f80_1810 => self.gpu.gp0_write(value),
            0x1f80_1814 => self.gpu.execute_gp1_command(value),
            0x1f80_1c00...0x1f80_1fff => (), //println!("[MMU] [INFO] Store to SPU region address: 0x{:08x}", address),
            0x1f80_2000...0x1f80_207f => (), //println!("[BUS] [INFO] Store to EXPENSION_2 region address: 0x{:08x}", address),
            0xfffe_0130 => (), //println!("[BUS] [INFO] Store to CACHE_CTRL region address: 0x{:08x}", address),
            _ => panic!("[BUS] [ERROR] Store to unrecognised address 0x{:08x}", address),
        };
    }

    fn execute_dma(&mut self, port: DmaPort) {
        let channel = self.dma.channel(port);
        let sync = channel.sync();

        match sync {
            SyncMode::Manual => self.execute_dma_manual(port),
            SyncMode::Request => self.execute_dma_request(port),
            SyncMode::LinkedList => self.execute_dma_linked_list(port),
        };
    }

    fn execute_dma_manual(&mut self, port: DmaPort) {
        let channel = self.dma.channel_mut(port);

        let mut address = channel.base_address() & 0x00ff_ffff;
        let mut remaining = channel.block_size();

        let step = channel.step();
        let direction = channel.direction();

        match direction {
            Direction::ToRam => {
                match port {
                    DmaPort::OTC => {
                        while remaining > 0 {
                            let value = match remaining {
                                1 => 0x00ff_ffff,
                                _ => address.wrapping_sub(4) & 0x00ff_ffff,
                            };

                            LittleEndian::write_u32(&mut self.ram[address as usize..], value);

                            address = value;
                            remaining -= 1;
                        }
                    },
                    _ => panic!("[DMA] [ERROR] Unsupported DMA Port {:?} for Manual", port),
                }
            },
            Direction::FromRam => {
                match port {
                    DmaPort::GPU => {
                        while remaining > 0 {
                            let data = LittleEndian::read_u32(&self.ram[address as usize..]);

                            self.gpu.gp0_write(data);

                            address = match step {
                                Step::Forward => address.wrapping_add(4),
                                Step::Backward => address.wrapping_sub(4),
                            } & 0x00ff_ffff;

                            remaining -= 1;
                        }
                    },
                    _ => panic!("[DMA] [ERROR] Unsupported DMA Port {:?} for Manual", port),
                }
            }
        };

        channel.finish();
    }
    
    fn execute_dma_request(&mut self, port: DmaPort) {
        let channel = self.dma.channel_mut(port);

        let mut address = channel.base_address() & 0x00ff_ffff;
        let mut remaining = channel.block_amount() * channel.block_size();

        let step = channel.step();
        let direction = channel.direction();

        match direction {
            Direction::ToRam => {
                match port {
                    DmaPort::GPU => {
                        while remaining > 0 {
                            LittleEndian::write_u32(&mut self.ram[address as usize..], self.gpu.gpuread());

                            address = match step {
                                Step::Forward => address.wrapping_add(4),
                                Step::Backward => address.wrapping_sub(4),
                            } & 0x00ff_ffff;

                            remaining -= 1;
                        }
                    },
                    _ => panic!("[DMA] [ERROR] Unsupported DMA Port {:?} for Request", port),
                }
            },
            Direction::FromRam => {
                match port {
                    DmaPort::GPU => {
                        while remaining > 0 {
                            let data = LittleEndian::read_u32(&self.ram[address as usize..]);

                            self.gpu.gp0_write(data);

                            address = match step {
                                Step::Forward => address.wrapping_add(4),
                                Step::Backward => address.wrapping_sub(4),
                            } & 0x00ff_ffff;

                            remaining -= 1;
                        }
                    },
                    _ => panic!("[DMA] [ERROR] Unsupported DMA Port {:?} for Request", port),
                }
            }
        };

        channel.finish();
    }

    fn execute_dma_linked_list(&mut self, port: DmaPort) {
        let channel = self.dma.channel_mut(port);

        let mut address = channel.base_address() & 0x00ff_ffff;

        let direction = channel.direction();

        match direction {
            Direction::ToRam => panic!("[DMA] [ERROR] Unsupported DMA Direction for LinkedList {:?}", direction),
            Direction::FromRam => {
                match port {
                    DmaPort::GPU => {
                        while address != 0x00ff_ffff {
                            let header = LittleEndian::read_u32(&self.ram[address as usize..]);

                            let mut payload_length = header >> 24;

                            while payload_length > 0 {
                                address = (address + 4) & 0x00ff_ffff;

                                let command = LittleEndian::read_u32(&self.ram[address as usize..]);

                                self.gpu.gp0_write(command);

                                payload_length -= 1;
                            }

                            address = header & 0x00ff_ffff;
                        }
                    }
                    _ => panic!("[DMA] [ERROR] Unsupported DMA Port {:?} for LinkedList", port),
                }
            },
        };

        channel.finish();
    }

    fn debug_fetch(&self, address: u32) -> Result<u32, ()> {
        match address {
            0x0000_0000...0x0020_0000 => Ok(LittleEndian::read_u32(&self.ram[address as usize..])),
            0x1fc0_0000...0x1fc8_0000 => Ok(LittleEndian::read_u32(&self.bios[address as usize - 0x1fc0_0000..])),
            _ => Err(()),
        }
    }

    pub fn debug_load(&self, width: BusWidth, address: u32) -> Result<u32, ()> {
        use self::BusWidth::*;

        let mask = match width {
            BYTE => 0x0000_00ff,
            HALF => 0x0000_ffff,
            WORD => 0xffff_ffff,
        };

        let fetch = self.debug_fetch(address);

        if fetch.is_ok() {
            Ok(fetch.unwrap() & mask)
        } else {
            Err(())
        }
    }
}