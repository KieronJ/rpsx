use byteorder::{ByteOrder, LittleEndian};
use serde::{Deserialize, Serialize};

use crate::util;

use super::cdrom::Cdrom;
use super::exp2::Exp2;
use super::gpu::Gpu;
use super::intc::Intc;
use super::sio0::Sio0;
use super::mdec::Mdec;
use super::spu::Spu;
use super::timekeeper::{Device, Timekeeper};
use super::timers::Timers;

#[derive(PartialEq)]
pub enum BusWidth {
    BYTE,
    HALF,
    WORD,
}

#[derive(Deserialize, Serialize)]
pub struct Bus {
    bios: Box<[u8]>,
    ram: Box<[u8]>,
    scratchpad: Box<[u8]>,

    cdrom: Cdrom,
    gpu: Gpu,
    mdec: Mdec,
    sio0: Sio0,
    spu: Spu,

    exp2: Exp2,

    intc: Intc,

    timers: Timers,
}

impl Bus {
    pub fn new(bios_filepath: &str, game_filepath: &str) -> Bus {
        let mut bios = util::read_file_to_box(bios_filepath);

        /* Enable TTY output */
        bios[0x6f0c] = 0x01;
        bios[0x6f0d] = 0x00;
        bios[0x6f0e] = 0x01;
        bios[0x6f0f] = 0x24;
        bios[0x6f14] = 0xc0;
        bios[0x6f15] = 0xa9;
        bios[0x6f16] = 0x81;
        bios[0x6f17] = 0xaf;

        /* Fast boot */
        //bios[0x18000] = 0x08;
        //bios[0x18001] = 0x00;
        //bios[0x18002] = 0xe0;
        //bios[0x18003] = 0x03;
        //bios[0x18004] = 0x00;
        //bios[0x18005] = 0x00;
        //bios[0x18006] = 0x00;
        //bios[0x18007] = 0x00;

        Bus {
            bios: bios,
            ram: vec![0; 0x200000].into_boxed_slice(),
            scratchpad: vec![0; 0x400].into_boxed_slice(),

            cdrom: Cdrom::new(game_filepath),
            gpu: Gpu::new(),
            mdec: Mdec::new(),
            sio0: Sio0::new(),
            spu: Spu::new(),

            exp2: Exp2::new(),

            intc: Intc::new(),

            timers: Timers::new(),
        }
    }

    pub fn get_address(&self) -> u64 {
        self as *const _ as u64
    }

    pub fn get_recompiler_store_word() -> u64 {
        Self::recompiler_store_word as *const () as u64
    }

    pub fn reset(&mut self) {
        self.cdrom.reset();
        self.sio0.reset();
    }

    pub fn ram(&mut self) -> &mut Box<[u8]> {
        &mut self.ram
    }

    pub fn cdrom(&mut self) -> &mut Cdrom {
        &mut self.cdrom
    }

    pub fn gpu(&self) -> &Gpu {
        &self.gpu
    }

    pub fn gpu_mut(&mut self) -> &mut Gpu {
        &mut self.gpu
    }

    pub fn mdec(&mut self) -> &mut Mdec {
        &mut self.mdec
    }

    pub fn sio0(&mut self) -> &mut Sio0 {
        &mut self.sio0
    }

    pub fn spu(&mut self) -> &mut Spu {
        &mut self.spu
    }

    pub fn intc(&mut self) -> &mut Intc {
        &mut self.intc
    }

    pub fn tick_device_by_id(&mut self, device: Device, cycles: usize) {
        let intc = &mut self.intc;

        match device {
            Device::Gpu => self.gpu.tick(intc, &mut self.timers, cycles),
            Device::Cdrom => self.cdrom.tick(intc, &mut self.spu, cycles),
            Device::Spu => for _ in 0..cycles {
                self.spu.tick(intc);
            },
            Device::Timers => self.timers.tick(intc, cycles),
            Device::Sio0 => self.sio0.tick(intc, cycles),
        };
    }

    pub unsafe fn load(&mut self, tk: &mut Timekeeper, width: BusWidth, address: u32) -> (u32, bool) {
        let mut error = false;

        let value = match address {
            0x0000_0000..=0x007f_ffff => {
                let offset = (address & 0x1f_ffff) as usize;

                match width {
                    BusWidth::BYTE => *self.ram.get_unchecked(offset) as u32,
                    BusWidth::HALF => {
                        let slice = self.ram.get_unchecked(offset & !0x1..);
                        LittleEndian::read_u16(slice) as u32
                    },
                    BusWidth::WORD => {
                        let slice = self.ram.get_unchecked(offset & !0x3..);
                        LittleEndian::read_u32(slice) as u32
                    },
                }
            },
            0x1f00_0000..=0x1f7f_ffff => 0xffff_ffff, //println!("[MMU] [INFO] Load from EXPENSION_1 region address: 0x{:08x}", address); 0xffff_ffff },
            0x1f80_0000..=0x1f80_03ff => {
                let offset = (address - 0x1f80_0000) as usize;

                match width {
                    BusWidth::BYTE => *self.scratchpad.get_unchecked(offset) as u32,
                    BusWidth::HALF => {
                        let slice = self.scratchpad.get_unchecked(offset & !0x1..);
                        LittleEndian::read_u16(slice) as u32
                    },
                    BusWidth::WORD => {
                        let slice = self.scratchpad.get_unchecked(offset & !0x3..);
                        LittleEndian::read_u32(slice)
                    },
                }
            },
            0x1f80_1014 => 0x2009_31e1,
            0x1f80_1060 => 0x0000_0b88,
            0x1f80_1040 => {
                tk.sync_device(self, Device::Sio0);
                self.sio0.rx_data()
            },
            0x1f80_1044 => {
                tk.sync_device(self, Device::Sio0);
                self.sio0.status()
            }
            //0x1f80_1048 => {
            //    tk.sync_device(self, Device::Gpu);
            //    tk.sync_device(self, Device::Sio0);
            //    self.sio0.read_mode()
            //}
            0x1f80_104a => {
                tk.sync_device(self, Device::Sio0);
                self.sio0.read_control()
            }
            0x1f80_104e => {
                tk.sync_device(self, Device::Sio0);
                self.sio0.read_baud()
            }
            0x1f80_1070 => self.intc.read_status(),
            0x1f80_1074 => self.intc.read_mask(),
            0x1f80_1100..=0x1f80_112b => {
                tk.sync_device(self, Device::Timers);
                self.timers.read(address)
            }
            0x1f80_1800..=0x1f80_1803 => {
                tk.sync_device(self, Device::Cdrom);

                if address == 0x1f80_1802 && width == BusWidth::HALF {
                    self.cdrom.read_data_half() as u32
                } else {
                    self.cdrom.read(address) as u32
                }
            }
            0x1f80_1810 => {
                tk.sync_device(self, Device::Gpu);
                self.gpu.gpuread()
            }
            0x1f80_1814 => {
                tk.sync_device(self, Device::Gpu);
                self.gpu.gpustat()
            }
            0x1f80_1820 => self.mdec.read_data(),
            0x1f80_1824 => self.mdec.read_status(),
            0x1f80_1c00..=0x1f80_1fff => {
                tk.sync_device(self, Device::Cdrom);
                tk.sync_device(self, Device::Spu);

                match width {
                    BusWidth::BYTE => self.spu.read16(address & !0x1) as u32,
                    BusWidth::HALF => self.spu.read16(address) as u32,
                    BusWidth::WORD => self.spu.read32(address) as u32,
                }
            },
            0x1f80_2000..=0x1f80_207f => self.exp2.read8(address) as u32,
            0x1fc0_0000..=0x1fc7_ffff => {
                let offset = (address - 0x1fc0_0000) as usize;

                match width {
                    BusWidth::BYTE => *self.bios.get_unchecked(offset) as u32,
                    BusWidth::HALF => {
                        let slice = self.bios.get_unchecked(offset & !0x1..);
                        LittleEndian::read_u16(slice) as u32
                    },
                    BusWidth::WORD => {
                        let slice = self.bios.get_unchecked(offset & !0x3..);
                        LittleEndian::read_u32(slice) as u32
                    },
                }
            },
            _ => { error = true; 0 },
        };

        (value, error)
    }

    pub fn load_instruction(&mut self, address: u32) -> u32 {
        if (address & 0x3) != 0 {
            panic!("[RECOMPILER] [ERROR] Unaligned address: 0x{:08x}", address);
        }

        match address {
            0x0000_0000..=0x007f_ffff => {
                let offset = (address & 0x1f_fffc) as usize;
                LittleEndian::read_u32(&self.ram[offset..])
            },
            0x1f80_0000..=0x1f80_03ff => {
                let offset = ((address - 0x1f80_0000) & !0x3) as usize;
                LittleEndian::read_u32(&self.scratchpad[offset..])
            },
            0x1fc0_0000..=0x1fc7_ffff => {
                let offset = (address as usize - 0x1fc0_0000) & !0x3;
                LittleEndian::read_u32(&self.bios[offset..])
            },
            _ => panic!("[RECOMPILER] [ERROR] Unrecognised address: 0x{:08x}", address),
        }
    }

    pub unsafe fn store(&mut self, tk: &mut Timekeeper, width: BusWidth, address: u32, value: u32) -> bool {
        let mut error = false;

        match address {
            0x0000_0000..=0x007f_ffff => {
                let offset = (address & 0x1f_ffff) as usize;

                match width {
                    BusWidth::BYTE => *self.ram.get_unchecked_mut(offset) = value as u8,
                    BusWidth::HALF => {
                        let slice = self.ram.get_unchecked_mut(offset & !0x1..);
                        LittleEndian::write_u16(slice, value as u16);
                    }
                    BusWidth::WORD => {
                        let slice = self.ram.get_unchecked_mut(offset & !0x3..);
                        LittleEndian::write_u32(slice, value);
                    }
                }
            }
            0x1f00_0000..=0x1f7f_ffff => (), //println!("[MMU] [INFO] Store to EXPENSION_1 region address: 0x{:08x}", address);
            0x1f80_0000..=0x1f80_03ff => {
                let offset = (address - 0x1f80_0000) as usize;

                match width {
                    BusWidth::BYTE => *self.scratchpad.get_unchecked_mut(offset) = value as u8,
                    BusWidth::HALF => {
                        let slice = &mut self.scratchpad.get_unchecked_mut(offset & !0x1..);
                        LittleEndian::write_u16(slice, value as u16);
                    },
                    BusWidth::WORD => {
                        let slice = &mut self.scratchpad.get_unchecked_mut(offset & !0x3..);
                        LittleEndian::write_u32(slice, value);
                    },
                }
            }
            0x1f80_1000..=0x1f80_1023 => (), //println!("[BUS] [INFO] Store to MEM_CTRL region address: 0x{:08x}", address),
            0x1f80_1040 => {
                tk.sync_device(self, Device::Gpu);
                tk.sync_device(self, Device::Sio0);
                self.sio0.tx_data(value)
            }
            0x1f80_1048 => {
                tk.sync_device(self, Device::Gpu);
                tk.sync_device(self, Device::Sio0);
                self.sio0.write_mode(value as u16)
            }
            0x1f80_104a => {
                tk.sync_device(self, Device::Gpu);
                tk.sync_device(self, Device::Sio0);
                self.sio0.write_control(value as u16)
            }
            0x1f80_104e => {
                tk.sync_device(self, Device::Gpu);
                tk.sync_device(self, Device::Sio0);
                self.sio0.write_baud(value as u16)
            }
            0x1f80_1060 => (), //println!("[BUS] [INFO] Store to MEM_CTRL region address: 0x{:08x}", address),
            0x1f80_1070 => self.intc.acknowledge_irq(value),
            0x1f80_1074 => self.intc.write_mask(value),
            0x1f80_1100..=0x1f80_112b => {
                tk.sync_device(self, Device::Timers);
                self.timers.write(address, value)
            }
            0x1f80_1800..=0x1f80_1803 => {
                tk.sync_device(self, Device::Cdrom);
                self.cdrom.write(address, value as u8)
            }
            0x1f80_1810 => {
                tk.sync_device(self, Device::Gpu);
                self.gpu.gp0_write(value)
            }
            0x1f80_1814 => {
                tk.sync_device(self, Device::Gpu);
                self.gpu.execute_gp1_command(value)
            }
            0x1f80_1820 => self.mdec.write_command(value),
            0x1f80_1824 => self.mdec.write_control(value),
            0x1f80_1c00..=0x1f80_1fff => {
                tk.sync_device(self, Device::Cdrom);
                tk.sync_device(self, Device::Spu);

                match width {
                    BusWidth::HALF => self.spu.write16(address, value as u16),
                    _ => panic!("[BUS] [ERROR] Unsupported SPU width"),
                }
            },
            0x1f80_2000..=0x1f80_207f => self.exp2.write8(address, value as u8),
            _ => {
                error = true;
                //println!("[BUS] [ERROR] Store to unrecognised address 0x{:08x}", address)
            },
        };

        error
    }

    pub fn recompiler_store_word(&mut self, address: u32, value: u32) {

    }
}
