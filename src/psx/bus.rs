use byteorder::{LittleEndian, ByteOrder};

use std::cell::RefCell;
use std::rc::Rc;

use util;

use super::cdrom::Cdrom;
use super::controller::Controller;
use super::dma::{Direction, Dma, DmaPort, Step, SyncMode};
use super::gpu::Gpu;
use super::interrupt::{Interrupt, InterruptRegister};
use super::joypad::Joypad;
use super::mdec::Mdec;
use super::spu::Spu;
use super::timer::Timer;

pub enum BusWidth {
    BYTE,
    HALF,
    WORD,
}

pub struct Bus {
    bios: Box<[u8]>,
    ram: Box<[u8]>,
    scratchpad: Box<[u8]>,
    
    cdrom: Cdrom,
    dma: Dma,
    gpu: Gpu,
    joypad: Joypad,
    mdec: Mdec,
    spu: Spu,

    istat: Rc<RefCell<InterruptRegister>>,
    imask: InterruptRegister,

    timer: Rc<RefCell<Timer>>,
}

impl Bus {
    pub fn new(bios_filepath: &str, game_filepath: &str) -> Bus {
        let controller = Rc::new(RefCell::new(Controller::new()));
        let istat = Rc::new(RefCell::new(InterruptRegister::new()));
        let timer = Rc::new(RefCell::new(Timer::new(istat.clone())));

        Bus {
            bios: util::read_file_to_box(bios_filepath),
            ram: vec![0; 0x200000].into_boxed_slice(),
            scratchpad: vec![0; 0x400].into_boxed_slice(),

            cdrom: Cdrom::new(game_filepath),
            dma: Dma::new(),
            gpu: Gpu::new(controller.clone(), timer.clone()),
            joypad: Joypad::new(controller),
            mdec: Mdec::new(),
            spu: Spu::new(),

            istat: istat,
            imask: InterruptRegister::new(),

            timer: timer,
        }
    }

    pub fn cdrom(&mut self) -> &mut Cdrom {
        &mut self.cdrom
    }

    pub fn dma(&mut self) -> &mut Dma {
        &mut self.dma
    }

    pub fn gpu(&mut self) -> &mut Gpu {
        &mut self.gpu
    }

    pub fn joypad(&mut self) -> &mut Joypad {
        &mut self.joypad
    }

    pub fn tick_timers(&mut self, clocks: usize) {
        let mut timer = self.timer.borrow_mut();

        timer.tick0(clocks);
        timer.tick1(clocks);
        timer.tick2(clocks);
    }

    pub fn check_interrupts(&self) -> bool {
        let istat = self.istat.borrow();

        (istat.read() & self.imask.read()) != 0
    }

    pub fn set_interrupt(&mut self, interrupt: Interrupt) {
        let mut istat = self.istat.borrow_mut();

        istat.set_interrupt(interrupt);
    }

    pub fn load(&mut self, address: u32, half: bool) -> u32 {
        match address {
            0x0000_0000...0x007f_ffff => LittleEndian::read_u32(&self.ram[(address & 0x1f_ffff) as usize..]),
            0x1f00_0000...0x1f07_ffff => { 0xffff_ffff }, //println!("[MMU] [INFO] Load from EXPENSION_1 region address: 0x{:08x}", address); 0xffff_ffff },
            0x1f80_0000...0x1f80_03ff => LittleEndian::read_u32(&self.scratchpad[(address - 0x1f80_0000) as usize..]),
            0x1f80_1014 => 0x2009_31e1,
            0x1f80_1060 => 0x0000_0b88,
            0x1f80_1040 => self.joypad.rx_data(),
            0x1f80_1044 => self.joypad.status(),
            //0x1f80_1048 => self.joypad.read_mode(),
            0x1f80_104a => self.joypad.read_control(),
            0x1f80_104e => self.joypad.read_baud(),
            0x1f80_1070 => self.istat.borrow().read(),
            0x1f80_1074 => self.imask.read(),
            0x1f80_1080...0x1f80_10ff => self.dma.read(address),
            0x1f80_1100...0x1f80_112b => self.timer.borrow_mut().read(address),
            0x1f80_1800...0x1f80_1803 => {
                if address == 0x1f80_1802 && half {
                    self.cdrom.read_data_half() as u32
                } else {
                    self.cdrom.read(address) as u32
                }
            },
            0x1f80_1810 => self.gpu.gpuread(),
            0x1f80_1814 => self.gpu.gpustat(),
            0x1f80_1820 => self.mdec.read_data(),
            0x1f80_1824 => self.mdec.read_status(),
            0x1f80_1c00...0x1f80_1fff => self.spu.read(address),
            0x1fc0_0000...0x1fc7_ffff => LittleEndian::read_u32(&self.bios[address as usize - 0x1fc0_0000..]),
            _ => panic!("[WARN] [ERROR] Load from unrecognised address 0x{:08x}", address),
        }
    }

    pub fn store(&mut self, width: BusWidth, address: u32, value: u32) {
        use self::BusWidth::*;

        match address {
            0x0000_0000...0x007f_ffff => {
                let slice = &mut self.ram[(address & 0x1f_ffff) as usize..];

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
            0x1f80_0000...0x1f80_03ff => {
                let slice = &mut self.scratchpad[(address - 0x1f80_0000) as usize..];

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
            0x1f80_1040 => self.joypad.tx_data(value),
            0x1f80_1048 => self.joypad.write_mode(value as u16),
            0x1f80_104a => self.joypad.write_control(value as u16),
            0x1f80_104e => self.joypad.write_baud(value as u16),
            0x1f80_1060 => (),//println!("[BUS] [INFO] Store to MEM_CTRL region address: 0x{:08x}", address),
            0x1f80_1070 => {
                let mut istat = self.istat.borrow_mut();

                let status = istat.read();
                istat.write(status & value);
            },
            0x1f80_1074 => self.imask.write(value),
            0x1f80_1080...0x1f80_10ff => {
                if let Some(port) = self.dma.write(address, value) {
                    //println!("[DMA] [INFO] Executing DMA\nPort: {:?}", port);
                    self.execute_dma(port);
                }
            },
            0x1f80_1100...0x1f80_112b => self.timer.borrow_mut().write(address, value),
            0x1f80_1800...0x1f80_1803 => self.cdrom.write(address, value as u8),
            0x1f80_1810 => self.gpu.gp0_write(value),
            0x1f80_1814 => self.gpu.execute_gp1_command(value),
            0x1f80_1820 => self.mdec.write_command(value),
            0x1f80_1824 => self.mdec.write_control(value),
            0x1f80_1c00...0x1f80_1fff => self.spu.write(address, value),
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

        self.dma.finish_set_interrupt(port);
    }

    fn execute_dma_manual(&mut self, port: DmaPort) {
        let channel = self.dma.channel_mut(port);

        let mut address = channel.base_address() & 0x1f_fffc;
        let mut remaining = channel.block_size();

        let step = channel.step();
        let direction = channel.direction();

        match direction {
            Direction::ToRam => {
                match port {
                    DmaPort::CDROM => {
                        while remaining > 0 {
                            let data = self.cdrom.data_dma();

                            LittleEndian::write_u32(&mut self.ram[address as usize..], data);

                            address = match step {
                                Step::Forward => address.wrapping_add(4),
                                Step::Backward => address.wrapping_sub(4),
                            } & 0x1f_fffc;

                            remaining -= 1;
                        }
                    },
                    DmaPort::OTC => {
                        while remaining > 0 {
                            let value = match remaining {
                                1 => 0xff_ffff,
                                _ => address.wrapping_sub(4) & 0x1f_fffc,
                            };

                            LittleEndian::write_u32(&mut self.ram[address as usize..], value);

                            address = value;
                            remaining -= 1;
                        }
                    },
                    _ => panic!("[DMA] [ERROR] Unsupported DMA Port {:?} for Manual", port),
                };
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
                            } & 0x1f_fffc;

                            remaining -= 1;
                        }
                    },
                    _ => panic!("[DMA] [ERROR] Unsupported DMA Port {:?} for Manual", port),
                };
            },
        };

        channel.finish();
    }
    
    fn execute_dma_request(&mut self, port: DmaPort) {
        let channel = self.dma.channel_mut(port);

        let mut address = channel.base_address() & 0x1f_fffc;
        let mut remaining = channel.block_amount() * channel.block_size();

        let step = channel.step();
        let direction = channel.direction();

        match direction {
            Direction::ToRam => {
                match port {
                    DmaPort::MDECOut => {
                        while remaining > 0 {
                            //TODO: Read data from MDEC.

                            address = match step {
                                Step::Forward => address.wrapping_add(4),
                                Step::Backward => address.wrapping_sub(4),
                            } & 0x1f_fffc;

                            remaining -= 1;
                        }
                    }
                    DmaPort::GPU => {
                        while remaining > 0 {
                            LittleEndian::write_u32(&mut self.ram[address as usize..], self.gpu.gpuread());

                            address = match step {
                                Step::Forward => address.wrapping_add(4),
                                Step::Backward => address.wrapping_sub(4),
                            } & 0x1f_fffc;

                            remaining -= 1;
                        }
                    },
                    _ => panic!("[DMA] [ERROR] Unsupported DMA Port {:?} for Request", port),
                };
            },
            Direction::FromRam => {
                match port {
                    DmaPort::MDECIn => {
                        while remaining > 0 {
                            let data = LittleEndian::read_u32(&self.ram[address as usize..]);

                            self.mdec.write_command(data);

                            address = match step {
                                Step::Forward => address.wrapping_add(4),
                                Step::Backward => address.wrapping_sub(4),
                            } & 0x1f_fffc;

                            remaining -= 1;
                        }
                    },
                    DmaPort::GPU => {
                        while remaining > 0 {
                            let data = LittleEndian::read_u32(&self.ram[address as usize..]);

                            self.gpu.gp0_write(data);

                            address = match step {
                                Step::Forward => address.wrapping_add(4),
                                Step::Backward => address.wrapping_sub(4),
                            } & 0x1f_fffc;

                            remaining -= 1;
                        }
                    },
                    DmaPort::SPU => {
                        while remaining > 0 {
                            let data = LittleEndian::read_u32(&self.ram[address as usize..]);

                            self.spu.dma_write(data);

                            address = match step {
                                Step::Forward => address.wrapping_add(4),
                                Step::Backward => address.wrapping_sub(4),
                            } & 0x1f_fffc;

                            remaining -= 1;
                        }
                    },
                    _ => panic!("[DMA] [ERROR] Unsupported DMA Port {:?} for Request", port),
                };
            },
        };

        channel.finish();
    }

    fn execute_dma_linked_list(&mut self, port: DmaPort) {
        let channel = self.dma.channel_mut(port);

        let mut address = channel.base_address() & 0x1f_fffc;

        let direction = channel.direction();

        match direction {
            Direction::ToRam => panic!("[DMA] [ERROR] Unsupported DMA Direction for LinkedList {:?}", direction),
            Direction::FromRam => {
                match port {
                    DmaPort::GPU => {
                        loop {
                            let header = LittleEndian::read_u32(&self.ram[address as usize..]);

                            let mut payload_length = header >> 24;

                            for _ in 0..payload_length {
                                address = (address + 4) & 0x1f_fffc;

                                let command = LittleEndian::read_u32(&self.ram[address as usize..]);
                                self.gpu.gp0_write(command);
                            }

                            if (header & 0x80_0000) != 0 {
                                break;
                            }

                            address = header & 0x1f_fffc;
                        }
                    },
                    _ => panic!("[DMA] [ERROR] Unsupported DMA Port {:?} for LinkedList", port),
                };
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