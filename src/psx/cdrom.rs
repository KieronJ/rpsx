use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

use queue::Queue;
use util;

pub const SECTORS_PER_SECOND: u64 = 75;
pub const SECTORS_PER_MINUTE: u64 = 60 * SECTORS_PER_SECOND;
pub const BYTES_PER_SECTOR: u64 = 2352;
pub const LEAD_IN_SECTORS: u64 = 2 * SECTORS_PER_SECOND;

pub const ADDRESS_OFFSET: u64 = 12;
pub const DATA_OFFSET: u64 = 24;

#[derive(Clone, Copy, Debug)]
enum CdromIndex {
    Index0,
    Index1,
    Index2,
    Index3
}

#[derive(Clone, Copy, PartialEq)]
enum CdromControllerMode {
    Idle,
    ParameterTransfer,
    CommandTransfer,
    CommandExecute,
    ResponseClear,
    ResponseTransfer,
    InterruptTransfer,
}

#[derive(Clone, Copy, PartialEq)]
enum CdromDriveMode {
    Idle,
    GetID,
    GetStat,
    Read,
}

pub struct Cdrom {
    index: CdromIndex,

    interrupt_enable: u8,
    interrupt_flags: u8,

    command: Option<u8>,

    playing: bool,
    seeking: bool,
    reading: bool,

    parameter_buffer: Queue<u8>,
    response_buffer: Queue<u8>,
    data_buffer: Queue<u8>,

    want_data: bool,

    data_busy: bool,

    seek: bool,
    seek_minute: u8,
    seek_second: u8,
    seek_sector: u8,

    mode_double_speed: bool,
    mode_sector_size: bool,

    controller_counter: usize,
    controller_mode: CdromControllerMode,

    controller_interrupt_flags: u8,

    controller_command: u8,

    controller_parameter_buffer: Queue<u8>,
    controller_response_buffer: Queue<u8>,

    drive_counter: usize,
    drive_mode: CdromDriveMode,

    drive_seek_minute: u8,
    drive_seek_second: u8,
    drive_seek_sector: u8,

    game_file: File,
}

impl Cdrom {
    pub fn new(game_filepath: &str) -> Cdrom {
        Cdrom {
            index: CdromIndex::Index0,

            interrupt_enable: 0,
            interrupt_flags: 0,

            command: None,
            
            playing: false,
            seeking: false,
            reading: false,

            parameter_buffer: Queue::<u8>::new(16),
            response_buffer: Queue::<u8>::new(16),
            data_buffer: Queue::<u8>::new(4096),

            want_data: false,

            data_busy: false,

            seek: false,
            seek_minute: 0,
            seek_second: 0,
            seek_sector: 0,
            
            mode_double_speed: false,
            mode_sector_size: false,

            controller_counter: 0,
            controller_mode: CdromControllerMode::Idle,

            controller_interrupt_flags: 0,

            controller_command: 0,

            controller_parameter_buffer: Queue::<u8>::new(16),
            controller_response_buffer: Queue::<u8>::new(16),

            drive_counter: 0,
            drive_mode: CdromDriveMode::Idle,

            drive_seek_minute: 0,
            drive_seek_second: 0,
            drive_seek_sector: 0,

            game_file: File::open(game_filepath).unwrap(),
        }
    }

    pub fn tick(&mut self, clocks: usize) -> bool {
        self.tick_controller(clocks);
        self.tick_drive(clocks);

        if self.interrupt_flags != 0 {
            return (self.interrupt_enable & self.interrupt_flags) != 0;
        }

        false
    }

    fn tick_controller(&mut self, clocks: usize) {
        if self.controller_counter != 0 {
            if self.controller_counter < clocks {
                self.controller_counter = clocks;
            }

            self.controller_counter -= clocks;
            return;
        }

        match self.controller_mode {
            CdromControllerMode::Idle => {
                if let Some(_) = self.command {
                    if self.parameter_buffer.has_data() {
                        self.controller_mode = CdromControllerMode::ParameterTransfer;
                    } else {
                        self.controller_mode = CdromControllerMode::CommandTransfer;
                    }
                }

                self.controller_counter = 1000;
            },
            CdromControllerMode::ParameterTransfer => {
                if self.parameter_buffer.has_data() {
                    let parameter = self.parameter_buffer.pop();
                    self.controller_parameter_buffer.push(parameter);

                    self.controller_counter = 10;
                } else {
                    self.controller_mode = CdromControllerMode::CommandTransfer;
                    self.controller_counter = 1000;
                }
            },
            CdromControllerMode::CommandTransfer => {
                self.controller_command = self.command.unwrap();
                self.command = None;

                self.controller_mode = CdromControllerMode::CommandExecute;
                self.controller_counter = 1000;
            },
            CdromControllerMode::CommandExecute => {
                let command = self.controller_command;
                self.execute_command(command);
                self.controller_parameter_buffer.clear();

                self.controller_mode = CdromControllerMode::ResponseClear;
                self.controller_counter = 1000;
            },
            CdromControllerMode::ResponseClear => {
                if self.response_buffer.has_data() {
                    self.response_buffer.pop();

                    self.controller_counter = 10;
                } else {
                    self.controller_mode = CdromControllerMode::ResponseTransfer;
                    self.controller_counter = 1000;
                }
            },
            CdromControllerMode::ResponseTransfer => {
                if self.controller_response_buffer.has_data() {
                    let response = self.controller_response_buffer.pop();
                    self.response_buffer.push(response);

                    self.controller_counter = 10;
                } else {
                    self.controller_mode = CdromControllerMode::InterruptTransfer;
                    self.controller_counter = 1000;
                }
            },
            CdromControllerMode::InterruptTransfer => {
                if self.interrupt_flags == 0 {
                    self.interrupt_flags = self.controller_interrupt_flags;

                    self.controller_mode = CdromControllerMode::Idle;
                    self.controller_counter = 1;
                } else {
                    self.controller_counter = 1;
                }
            },
        }
    }

    fn tick_drive(&mut self, clocks: usize) {
        if self.drive_counter != 0 {
            if self.drive_counter < clocks {
                self.drive_counter = clocks;
            }

            self.drive_counter -= clocks;
            return;
        }

        match self.drive_mode {
            CdromDriveMode::Idle => (),
            CdromDriveMode::GetID => {
                if self.interrupt_flags == 0 {
                    self.controller_response_buffer.push(0x02);
                    self.controller_response_buffer.push(0x00);

                    self.controller_response_buffer.push(0x20);
                    self.controller_response_buffer.push(0x00);

                    self.controller_response_buffer.push('S' as u8);
                    self.controller_response_buffer.push('C' as u8);
                    self.controller_response_buffer.push('E' as u8);
                    self.controller_response_buffer.push('A' as u8);

                    self.controller_interrupt_flags = 0x2;

                    self.controller_mode = CdromControllerMode::ResponseClear;
                    self.controller_counter = 1000;

                    self.drive_mode = CdromDriveMode::Idle;
                    self.drive_counter = 1000;
                } else {
                    self.drive_counter = 1000;
                }
            },
            CdromDriveMode::GetStat => {
                if self.interrupt_flags == 0 {
                    self.push_stat();

                    self.controller_interrupt_flags = 0x2;

                    self.controller_mode = CdromControllerMode::ResponseClear;
                    self.controller_counter = 1000;

                    self.drive_mode = CdromDriveMode::Idle;
                    self.drive_counter = 1000;
                } else {
                    self.drive_counter = 1000;
                }
            },
            CdromDriveMode::Read => {
                if self.interrupt_flags == 0 {
                    self.push_stat();

                    if self.seek {
                        self.do_seek();
                    }

                    self.seeking = false;
                    self.reading = true;

                    self.data_busy = true;

                    let cursor = match self.mode_sector_size {
                        true => self.get_seek_location() + ADDRESS_OFFSET,
                        false => self.get_seek_location() + DATA_OFFSET,
                    };

                    self.game_file.seek(SeekFrom::Start(cursor)).unwrap();

                    //println!("MM: {}", self.drive_seek_minute);
                    //println!("SS: {}", self.drive_seek_second);
                    //println!("FF: {}", self.drive_seek_sector);

                    let sector_size = match self.mode_sector_size {
                        true => 2340,
                        false => 2048,
                    };

                    let mut sector = vec![0u8; sector_size];
                    self.game_file.read_exact(&mut sector).unwrap();

                    self.data_buffer.clear();

                    for i in 0..sector_size {
                        self.data_buffer.push(sector[i]);
                    }
                    
                    self.drive_seek_sector += 1;

                    if self.drive_seek_sector >= 75 {
                        self.drive_seek_sector = 0;
                        self.drive_seek_second += 1;
                    }

                    if self.drive_seek_second >= 60 {
                        self.drive_seek_second = 0;
                        self.drive_seek_minute += 1;
                    }

                    self.controller_interrupt_flags = 0x1;

                    self.controller_mode = CdromControllerMode::ResponseClear;
                    self.controller_counter = 1000;

                    self.drive_counter = 16934400 / match self.mode_double_speed {
                        true => 150,
                        false => 75,
                    };
                } else {
                    self.drive_counter = 10;
                }
            }
        };
    }

    fn execute_command(&mut self, command: u8) {
        match command {
            0x01 => {
                //println!("[CDROM] [INFO] GetStat");
                self.push_stat();
            },
            0x02 => {
                //println!("[CDROM] [INFO] SetLoc");
                self.push_stat();

                let minute = self.controller_parameter_buffer.pop();
                let second = self.controller_parameter_buffer.pop();
                let sector = self.controller_parameter_buffer.pop();

                self.seek = true;
                self.seek_minute = util::bcd_to_u8(minute);
                self.seek_second = util::bcd_to_u8(second);
                self.seek_sector = util::bcd_to_u8(sector);
            },
            0x06 => {
                //println!("[CDROM] [INFO] ReadN");

                self.reading = true;
                self.seeking = false;
                self.playing = false;

                self.push_stat();

                self.drive_mode = CdromDriveMode::Read;
                self.drive_counter = 33868800 / match self.mode_double_speed {
                    true => 150,
                    false => 75,
                };
            },
            0x09 => {
                //println!("[CDROM] [INFO] Pause");
                self.push_stat();

                self.playing = false;
                self.reading = false;
                self.seeking = false;

                self.drive_mode = CdromDriveMode::GetStat;
                self.drive_counter = 1;
            },
            0x0a => {
                //println!("[CDROM] [INFO] Init");
                self.push_stat();
                
                self.mode_double_speed = false;
                self.mode_sector_size = false;
                self.reading = false;
                self.playing = false;
                self.seeking = false;

                self.drive_mode = CdromDriveMode::GetStat;
                self.drive_counter = 1000;
            },
            0x0b => {
                //println!("[CDROM] [INFO] Mute");
                self.push_stat();
            }
            0x0c => {
                //println!("[CDROM] [INFO] Demute");
                self.push_stat();
            }
            0x0d => {
                println!("[CDROM] [INFO] SetFilter");

                self.push_stat();
            }
            0x0e => {
                //println!("[CDROM] [INFO] SetMode");
                self.push_stat();

                let mode = self.controller_parameter_buffer.pop();

                self.mode_double_speed =  (mode & 0x80) != 0;
                self.mode_sector_size = (mode & 0x20) != 0;

                if (mode & 0x10) != 0 {
                    panic!("Force");
                }

                if (mode & 0x04) != 0 {
                    println!("[CDROM] [INFO] Report requested.");
                }

            },
            0x13 => {
                println!("[CDROM] [INFO] GetTN");
                self.push_stat();
                self.controller_response_buffer.push(1);
                self.controller_response_buffer.push(23);
            },
            0x15 => {
                //println!("[CDROM] [INFO] SeekL");
                self.push_stat();

                self.do_seek();

                self.data_busy = false;

                self.drive_mode = CdromDriveMode::GetStat;
                self.drive_counter = 40000;
            },
            0x19 => {
                self.execute_test_command();
            },
            0x1a => {
                //println!("[CDROM] [INFO] GetID");
                self.push_stat();

                self.drive_mode = CdromDriveMode::GetID;
                self.drive_counter = 40000;
            },
            0x1b => {
                //println!("[CDROM] [INFO] ReadS");
                self.reading = true;
                self.seeking = false;
                self.playing = false;

                self.push_stat();

                self.drive_mode = CdromDriveMode::Read;
                self.drive_counter = 33868800 / match self.mode_double_speed {
                    true => 150,
                    false => 75,
                };
            },
            0x1e => {
                //println!("[CDROM] [INFO] ReadTOC");
                self.push_stat();

                self.drive_mode = CdromDriveMode::GetStat;
                self.drive_counter = 33868800;
            }
            _ => panic!("[CDROM] [ERROR] Unknown command 0x{:02x}", command),
        };

        self.controller_interrupt_flags = 0x3;
    }

    fn execute_test_command(&mut self) {
        let command = self.controller_parameter_buffer.pop();

        match command {
            0x20 => {
                //println!("[CDROM] [INFO] GetVersion");

                self.controller_response_buffer.push(0x97);
                self.controller_response_buffer.push(0x01);
                self.controller_response_buffer.push(0x10);
                self.controller_response_buffer.push(0xc2);
            },
            _ => panic!("[CDROM] [ERROR] Unknown test command 0x{:02x}", command),
        }
    }

    fn do_seek(&mut self) {
        self.seek = false;
        self.seeking = true;
        self.reading = false;
        self.playing = false;
        self.drive_seek_minute = self.seek_minute;
        self.drive_seek_second = self.seek_second;
        self.drive_seek_sector = self.seek_sector;
    }

    fn get_seek_location(&self) -> u64 {
        let sector = ((self.drive_seek_minute as u64) * SECTORS_PER_MINUTE) +
                     ((self.drive_seek_second as u64) * SECTORS_PER_SECOND) +
                       self.drive_seek_sector as u64 - LEAD_IN_SECTORS;

        sector * BYTES_PER_SECTOR
    }

    fn get_stat(&self) -> u8 {
        let mut stat = 0;

        stat |= (self.playing as u8) << 7;
        stat |= (self.seeking as u8) << 6;
        stat |= (self.reading as u8) << 5;
        stat |= 0x2;

        stat
    }

    fn push_stat(&mut self) {
        let stat = self.get_stat();
        self.controller_response_buffer.push(stat);
    }

    pub fn busy(&self) -> bool {
        if self.controller_mode != CdromControllerMode::Idle {
            return true;
        }

        false
    }

    pub fn read(&mut self, address: u32) -> u8 {
        use self::CdromIndex::*;

        let mut value = 0;

        match address & 0x3 {
            0 => {
                value |= (self.busy() as u8) << 7;
                value |= (self.data_busy as u8) << 6;
                value |= (self.response_buffer.has_data() as u8) << 5;
                value |= (self.parameter_buffer.has_space() as u8) << 4;
                value |= (self.parameter_buffer.empty() as u8) << 3;
                //value |= (self. as u8) << 2;
                value |= self.index as u8;
            },
            1 => {
                value = self.response_buffer.pop();
                //println!("[CDROM] [INFO] Response 0x{:02x}", value);
            },
            2 => {
                value = self.data_buffer.pop();
            },
            3 => {
                match self.index {
                    Index0 => value = 0xe0 | self.interrupt_enable,
                    Index1 => value = 0xe0 | self.interrupt_flags,
                    _ => panic!("[CDROM] [ERROR] Read from CDROM_REG_{}_{:?}", address & 0x3, self.index),
                }
            },
            _ => panic!("[CDROM] [ERROR] Read from CDROM_REG_{}_{:?}", address & 0x3, self.index),
        };

        value
    }

    fn read_data(&mut self) -> u8 {
        self.data_buffer.pop()
    }

    pub fn read_data_half(&mut self) -> u16 {
        let h0 = self.read_data() as u16;
        let h1 = self.read_data() as u16;

        (h1 << 8) | h0
    }

    pub fn data_dma(&mut self) -> u32 {
        let b0 = self.read_data() as u32;
        let b1 = self.read_data() as u32;
        let b2 = self.read_data() as u32;
        let b3 = self.read_data() as u32;

        (b3 << 24) | (b2 << 16) | (b1 << 8) | b0
    }

    pub fn write(&mut self, address: u32, value: u8) {
        use self::CdromIndex::*;

        match address & 0x3 {
            0 => {
                self.index = match value & 0x3 {
                    0 => Index0,
                    1 => Index1,
                    2 => Index2,
                    3 => Index3,
                    _ => unreachable!(),
                };
            },
            1 => {
                match self.index {
                    Index0 => {
                        self.command = Some(value);
                    },
                    Index3 => (), // Right-CD to Right-SPU
                    _ => panic!("[CDROM] [ERROR] Write to CDROM_REG_{}_{:?}", address & 0x3, self.index),
                }
            },
            2 => {
                match self.index {
                    Index0 => self.parameter_buffer.push(value),
                    Index1 => self.interrupt_enable = value & 0x1f,
                    Index2 => (), // Left-CD to Left-SPU
                    Index3 => (), // Right-CD to Left-SPU
                }
            },
            3 => {
                match self.index {
                    Index0 => {
                        self.want_data = (value & 0x80) != 0;
                    },
                    Index1 => {
                        self.interrupt_flags &= !(value & 0x1f);

                        if (value & 0x40) != 0 {
                            self.parameter_buffer.clear();
                        }
                    },
                    Index2 => (), // Left-CD to Right-SPU
                    Index3 => (), // Apply Volume Change
                }
            },
            _ => panic!("[CDROM] [ERROR] Write to CDROM_REG_{}_{:?}", address & 0x3, self.index),
        };
    }
}