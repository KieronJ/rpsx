use std::fs;
use std::io::prelude::{Read, Write};
use std::os::windows::prelude::FileExt;
use std::path;

pub const MEMORY_CARD_SIZE: usize = 0x20000;

pub struct MemoryCard {
    file: fs::File,
    cache: Box<[u8]>,
    dirty: bool,

    ack: bool,
    state: usize,

    sector: u16,
    sector_counter: usize,

    flag: u8,
    previous: u8,

    checksum: u8,
    checksum_match: bool,
}

impl MemoryCard {
    pub fn new(filepath: &'static str) -> MemoryCard {
        let mut card = MemoryCard {
            file: MemoryCard::get_card_file(filepath),
            cache: vec![0; MEMORY_CARD_SIZE].into_boxed_slice(),
            dirty: false,

            ack: false,
            state: 0,

            sector: 0,
            sector_counter: 0,

            flag: 0,
            previous: 0,

            checksum: 0,
            checksum_match: false,
        };

        card.load_cache();
        card
    }

    pub fn reset(&mut self) {
        self.flag = 0x08;
    }

    pub fn reset_device_state(&mut self) {
        self.ack = false;
        self.state = 0;
    }

    fn get_card_file(filepath: &'static str) -> fs::File {
        let path = path::Path::new(filepath);

        // This should only be None if the path is at the root right?
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("unable to create path to memory card file");
        }

        fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(filepath)
            .expect("unable to create/open memory card file")
    }

    fn load_cache(&mut self) {
        let metadata = self.file.metadata().unwrap();

        // Forgo loading card data if file is smaller than expected (presumably empty)
        if metadata.len() >= MEMORY_CARD_SIZE as u64 {
            self.file.read_exact(&mut self.cache).unwrap();
            self.dirty = false;
        }
    }

    fn flush_cache(&mut self) {
        self.file.seek_write(&self.cache, 0).unwrap();
        self.file.flush().unwrap();
        self.dirty = false;
    }

    fn read_cache(&self, index: usize) -> u8 {
        self.cache[index]
    }

    fn write_cache(&mut self, index: usize, value: u8) {
        self.cache[index] = value;
        self.dirty = true;
    }

    pub fn sync(&mut self) {
        if self.dirty {
            self.flush_cache();
        }
    }

    pub fn response(&mut self, command: u8) -> u8 {
        self.ack = true;
        let mut reply = 0xff;

        match self.state {
            0 => self.state = 1,
            1 => {
                reply = self.flag;
                self.sector_counter = 0;

                match command {
                    0x52 => self.state = 10,
                    0x53 => self.state = 2,
                    0x57 => self.state = 21,
                    _ => {
                        println!("[MCD] [WARN] Unrecognised command: {:#x}", command);

                        self.state = 0;
                        self.ack = false;
                    },
                };
            },
            2 => {
                reply = 0x5a;
                self.state = 3;
            },
            3 => {
                reply = 0x5d;
                self.state = 4;
            },
            4 => {
                reply = 0x5c;
                self.state = 5;
            },
            5 => {
                reply = 0x5d;
                self.state = 6;
            },
            6 => {
                reply = 0x04;
                self.state = 7;
            },
            7 => {
                reply = 0x00;
                self.state = 8;
            },
            8 => {
                reply = 0x00;
                self.state = 9;
            },
            9 => {
                reply = 0x80;
                self.state = 0;
            },
            10 => {
                reply = 0x5a;
                self.state = 11;
            },
            11 => {
                reply = 0x5d;
                self.state = 12;
            },
            12 => {
                reply = 0x00;
                self.sector &= 0xff;
                self.sector |= (command as u16) << 8;

                self.previous = command;
                self.checksum = command;

                self.state = 13;
            },
            13 => {
                reply = self.previous;

                self.sector &= 0xff00;
                self.sector |= command as u16;

                self.checksum ^= command;

                //println!("[MCD] [INFO] [R] Set sector to: {:#x}", self.sector);

                if self.sector > 0x3ff {
                    self.sector = 0xffff;
                }

                self.state = 14;
            },
            14 => {
                reply = 0x5c;
                self.state = 15;
            },
            15 => {
                reply = 0x5d;
                self.state = 16;
            },
            16 => {
                reply = (self.sector >> 8) as u8;
                self.state = 17;
            },
            17 => {
                reply = self.sector as u8;

                //println!("[MCD] [INFO] Reading sector: {:#x}", self.sector);

                if self.sector == 0xffff {
                    self.state = 0;
                    self.ack = false;
                } else {
                    self.state = 18;
                }
            },
            18 => {
                let sector_addr = (self.sector as usize) * 0x80;
                reply = self.read_cache(sector_addr + self.sector_counter);

                self.checksum ^= reply;

                self.sector_counter += 1;

                if self.sector_counter == 0x80 {
                    self.state = 19;
                }
            },
            19 => {
                reply = self.checksum;
                self.state = 20;
            },
            20 => {
                //println!("[MCD] [INFO] [R] finishing transfer");
                reply = 0x47;
                self.state = 0;
                self.ack = false;
            },
            21 => {
                self.flag &= !0x08;

                reply = 0x5a;
                self.state = 22;
            },
            22 => {
                reply = 0x5d;
                self.state = 23;
            },
            23 => {
                reply = 0x00;
                self.sector &= 0xff;
                self.sector |= (command as u16) << 8;

                self.previous = command;
                self.checksum = command;

                self.state = 24;
            },
            24 => {
                reply = self.previous;

                self.sector &= 0xff00;
                self.sector |= command as u16;

                self.previous = command;
                self.checksum ^= command;

                //println!("[MCD] [INFO] [W] Set sector to: {:#x}", self.sector);

                if self.sector > 0x3ff {
                    self.state = 0;
                    self.ack = false;
                } else {
                    self.state = 25;
                }
            },
            25 => {
                reply = self.previous;

                let sector_addr = (self.sector as usize) * 0x80;
                self.write_cache(sector_addr + self.sector_counter, command);

                self.previous = command;
                self.checksum ^= command;

                self.sector_counter += 1;

                if self.sector_counter == 0x80 {
                    self.state = 26;
                }
            },
            26 => {
                reply = self.previous;

                //println!("[MCD] [INFO] Written sector: {:#x}", self.sector);
                self.sync();

                self.checksum_match = self.checksum == command;
                self.state = 27;
            },
            27 => {
                reply = 0x5c;
                self.state = 28;
            },
            28 => {
                reply = 0x5d;
                self.state = 29;
            },
            29 => {
                if self.checksum_match {
                    reply = 0x47;
                } else {
                    println!("[MCD] [WARN] Checksum mismatch: {:#x}", self.checksum);
                    reply = 0x4e;
                }

                self.ack = false;
                self.state = 0;
            },
            _ => panic!("[MCD] [ERROR] Unknown state: {}", self.state),
        };

        reply
    }

    pub fn ack(&self) -> bool {
        self.ack
    }

    pub fn enable(&self) -> bool {
        self.state != 0
    }
}
