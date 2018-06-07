#[derive(Clone, Copy, Debug)]
enum CdromIndex {
    Index0,
    Index1,
    Index2,
    Index3
}

impl CdromIndex {
    fn from(value: u32) -> CdromIndex {
        use self::CdromIndex::*;

        match value & 0x3 {
            0 => Index0,
            1 => Index1,
            2 => Index2,
            3 => Index3,
            _ => unreachable!(),
        }
    }
}

struct CdromStat {
    play: bool,
    seek: bool,
    read: bool,
    shell_open: bool,
    id_error: bool,
    seek_error: bool,
    motor_enabled: bool,
    error: bool,
}

impl CdromStat {
    pub fn new() -> CdromStat {
        CdromStat {
            play: false,
            seek: false,
            read: false,
            shell_open: false,
            id_error: false,
            seek_error: false,
            motor_enabled: false,
            error: false,
        }
    }

    pub fn as_u8(&self) -> u8 {
        let mut value = 0;

        value |= (self.play as u8) << 7;
        value |= (self.seek as u8) << 6;
        value |= (self.read as u8) << 5;
        value |= (self.shell_open as u8) << 4;
        value |= (self.id_error as u8) << 3;
        value |= (self.seek_error as u8) << 2;
        value |= (self.motor_enabled as u8) << 1;
        value |= (self.error as u8) << 0;

        value | (1 << 6)
    }
}

struct CdromQueue {
    commands: [u8; 16],
    length: usize,
}

impl CdromQueue {
    pub fn new() -> CdromQueue {
        CdromQueue {
            commands: [0; 16],
            length: 0,
        }
    }

    pub fn push(&mut self, value: u8) {
        self.commands[self.length] = value;

        if !self.full() {
            self.length += 1;
        }
    }

    pub fn pop(&mut self) -> u8 {
        let value = self.commands[0];

        if !self.empty() {
            self.length -= 1;

            for i in 0..self.length {
                self.commands[i] = self.commands[i + 1];
            }
        }

        value
    }

    pub fn empty(&self) -> bool {
        self.length == 0
    }

    pub fn full(&self) -> bool {
        self.length == 15
    }

    pub fn clear(&mut self) {
        self.length = 0;
    }
}

pub struct Cdrom {
    index: CdromIndex,

    stat: CdromStat,

    parameter_buffer: CdromQueue,
    response_buffer: CdromQueue,

    interrupt_enable: u8,
    interrupt_flag: u8,
}

impl Cdrom {
    pub fn new() -> Cdrom {
        Cdrom {
            index: CdromIndex::Index0,

            stat: CdromStat::new(),

            parameter_buffer: CdromQueue::new(),
            response_buffer: CdromQueue::new(),

            interrupt_enable: 0,
            interrupt_flag: 0,
        }
    }

    fn execute_command(&mut self, command: u8) {
        self.response_buffer.clear();

        match command {
            0x01 => {
                println!("[CDROM] [INFO] GetStat");
                self.response_buffer.push(self.stat.as_u8());

                self.interrupt_flag = 0x3;
            },
            0x19 => {
                self.execute_test_command();
            },
            _ => panic!("[CDROM] [ERROR] Unknown command 0x{:x}", command),
        };

        self.parameter_buffer.clear();
    }

    fn execute_test_command(&mut self) {
        let command = self.parameter_buffer.pop();

        match command {
            0x20 => {
                println!("[CDROM] [INFO] GetBiosDateVersion");

                self.response_buffer.push(0x94);
                self.response_buffer.push(0x09);
                self.response_buffer.push(0x19);
                self.response_buffer.push(0xc0);

                self.interrupt_flag = 0x3;
            },
            _ => panic!("[CDROM] [ERROR] Unknown test command 0x{:x}", command),
        }
    }

    pub fn check_interrupts(&self) -> bool {
        if self.interrupt_flag == 0 {
            return false;
        }

        (self.interrupt_flag & self.interrupt_enable & 0x07) != 0
    }

    pub fn read_status_register(&self) -> u32 {
        let mut value = 0;

        value |= (!self.response_buffer.empty() as u32) << 5;
        value |= (!self.parameter_buffer.full() as u32) << 4;
        value |= (self.parameter_buffer.empty() as u32) << 3;
        value |= self.index as u32;

        value
    }

    pub fn write_index_register(&mut self, value: u32) {
        self.index = CdromIndex::from(value & 0x3);
    }

    pub fn read_register1(&mut self) -> u32 {
        let response = self.response_buffer.pop();

        println!("[CDROM] [INFO] Response -> 0x{:02x}", response);

        response as u32
    }

    pub fn write_register1(&mut self, value: u32) {
        use self::CdromIndex::*;

        match self.index {
            Index0 => self.execute_command(value as u8),
            _ => panic!("[CDROM] [ERROR] Store to CDROM_REG_1 {:?}", self.index),
        };
    }

    pub fn read_register2(&self) -> u32 {
        match self.index {
            _ => panic!("[CDROM] [ERROR] Read from CDROM_REG_2 {:?}", self.index),
        }
    }

    pub fn write_register2(&mut self, value: u32) {
        use self::CdromIndex::*;

        match self.index {
            Index0 => self.parameter_buffer.push(value as u8),
            Index1 => self.interrupt_enable = (value & 0x1f) as u8,
            _ => panic!("[CDROM] [ERROR] Store to CDROM_REG_2 {:?}", self.index),
        };
    }

    pub fn read_register3(&self) -> u32 {
        use self::CdromIndex::*;

        match self.index {
            Index1 => (0xe0 | self.interrupt_flag) as u32,
            _ => panic!("[CDROM] [ERROR] Read from CDROM_REG_3 {:?}", self.index),
        }
    }

    pub fn write_register3(&mut self, value: u32) {
        use self::CdromIndex::*;

        match self.index {
            Index1 => {
                let flags = value & 0x1f;
                self.interrupt_flag &= !flags as u8;

                if (value & 0x40) != 0 {
                    self.parameter_buffer.clear();
                }
            },
            _ => panic!("[CDROM] [ERROR] Store to CDROM_REG_3 {:?}", self.index),
        }
    }
}