use queue::Queue;

pub const BLOCK_DECOMPRESS_TICKS: usize = 3763;

pub const ZIGZAG_TABLE: [usize; 64] = [
     0,  1,  5,  6, 14, 15, 27, 28,
     2,  4,  7, 13, 16, 26, 29, 42,
     3,  8, 12, 17, 25, 30, 41, 43,
     9, 11, 18, 24, 31, 40, 44, 53,
    10, 19, 23, 32, 39, 45, 52, 54,
    20, 22, 33, 38, 46, 51, 55, 60,
    21, 34, 37, 47, 50, 56, 59, 61,
    35, 36, 48, 49, 57, 58, 62, 63,
];

#[derive(Clone, Copy)]
enum MdecDepth {
    MD4Bit,
    MD8Bit,
    MD24Bit,
    MD15Bit,
}

impl MdecDepth {
    fn from(value: u32) -> MdecDepth {
        use self::MdecDepth::*;

        match value & 0x3 {
            0 => MD4Bit,
            1 => MD8Bit,
            2 => MD24Bit,
            3 => MD15Bit,
            _ => unreachable!(),
        }
    }
}

pub struct Mdec {
    dma0_enable: bool,
    dma1_enable: bool,

    busy: bool,

    output_depth: MdecDepth,
    output_signed: bool,
    output_msb: bool,

    current_block: usize,

    command: usize,
    parameters_remaining: u16,

    colour: bool,

    luminance_quant_table: [u8; 64],
    luminance_index: usize,

    chrominance_quant_table: [u8; 64],
    chrominance_index: usize,

    scale_table: [u16; 64],
    scale_index: usize,

    y_block: [u8; 64],
    cb_block: [u8; 64],
    cr_block: [u8; 64],

    data_out: Queue<u32>,
    data_in: Queue<u32>,
}

impl Mdec {
    pub fn new() -> Mdec {
        Mdec {
            dma0_enable: false,
            dma1_enable: false,

            busy: false,

            output_depth: MdecDepth::MD4Bit,
            output_signed: false,
            output_msb: false,

            current_block: 0,

            command: 0,
            parameters_remaining: 0,

            colour: false,

            luminance_quant_table: [0; 64],
            luminance_index: 0,

            chrominance_quant_table: [0; 64],
            chrominance_index: 0,

            scale_table: [0; 64],
            scale_index: 0,

            y_block: [0; 64],
            cb_block: [0; 64],
            cr_block: [0; 64],

            data_out: Queue::<u32>::new(0x20),
            data_in: Queue::<u32>::new(0x20),
        }
    }

    pub fn read_data(&mut self) -> u32 {
        self.data_out.pop()
    }

    pub fn read_status(&self) -> u32 {
        let mut value = 0;

        value |= (self.data_out.empty() as u32) << 31;
        value |= (self.data_in.full() as u32) << 30;
        value |= (self.busy as u32) << 29;

        value |= ((self.dma0_enable & self.data_out.empty() && self.command != 0) as u32) << 28;
        value |= ((self.dma1_enable & !self.data_in.full()) as u32) << 27;

        value |= (self.output_depth as u32) << 25;
        value |= (self.output_signed as u32) << 24;
        value |= (self.output_msb as u32) << 23;

        value |= self.parameters_remaining as u32;

        value
    }

    pub fn write_command(&mut self, value: u32) {
        if self.busy {
            match self.command {
                2 => {
                    let b0 = (value >> 24) as u8;
                    let b1 = (value >> 16) as u8;
                    let b2 = (value >> 8) as u8;
                    let b3 = value as u8;

                    if !self.colour || (self.parameters_remaining > 0x10) {
                        self.luminance_quant_table[self.luminance_index + 0] = b0;
                        self.luminance_quant_table[self.luminance_index + 1] = b1;
                        self.luminance_quant_table[self.luminance_index + 2] = b2;
                        self.luminance_quant_table[self.luminance_index + 3] = b3;

                        self.luminance_index += 4;
                    } else {
                        self.chrominance_quant_table[self.chrominance_index + 0] = b0;
                        self.chrominance_quant_table[self.chrominance_index + 1] = b1;
                        self.chrominance_quant_table[self.chrominance_index + 2] = b2;
                        self.chrominance_quant_table[self.chrominance_index + 3] = b3;

                        self.chrominance_index += 4;
                    }

                    self.parameters_remaining -= 1;
                },
                3 => {
                    let h0 = (value >> 16) as u16;
                    let h1 = value as u16;

                    self.scale_table[self.scale_index + 0] = h0;
                    self.scale_table[self.scale_index + 1] = h1;

                    self.scale_index += 2;

                    self.parameters_remaining -= 1;
                },
                _ => unreachable!(),
            }

            if self.parameters_remaining == 0 {
                self.busy = false;
            }
            
        } else {
            self.command = (value >> 29) as usize;
            self.busy = true;

            match self.command {
                0 => self.command_invalid(value),
                2 => self.command_set_quant_table(value),
                3 => self.command_set_scale_table(value),
                4 => self.command_invalid(value),
                5 => self.command_invalid(value),
                6 => self.command_invalid(value),
                7 => self.command_invalid(value),
                _ => panic!("[MDEC] [ERROR] Unimplemented command {:#x}", self.command),
            };
        }
    }

    fn command_invalid(&mut self, value: u32) {
        self.copy_bits_to_status(value);

        self.busy = false;

        self.parameters_remaining = value as u16;
    }

    fn command_set_quant_table(&mut self, value: u32) {
        self.copy_bits_to_status(value);

        self.luminance_index = 0;
        self.chrominance_index = 0;

        if (value & 0x1) != 0 {
            self.colour = true;
            self.parameters_remaining = 0x20;
        } else {
            self.colour = false;
            self.parameters_remaining = 0x10;
        }
    }

    fn command_set_scale_table(&mut self, value: u32) {
        self.copy_bits_to_status(value);

        self.scale_index = 0;

        self.parameters_remaining = 0x10;
    }

    fn copy_bits_to_status(&mut self, value: u32) {
        self.output_depth = MdecDepth::from((value & 0x1800_0000) >> 27);
        self.output_signed = (value & 0x400_0000) != 0;
        self.output_msb = (value & 0x200_0000) != 0;
    }

    pub fn write_control(&mut self, value: u32) {
        if (value & 0x8000_0000) != 0 {
            self.reset();
        }

        if (value & 0x4000_0000) != 0 {
            self.dma0_enable = true;
        }

        if (value & 0x2000_0000) != 0 {
            self.dma1_enable = true;
        }
    }

    fn reset(&mut self) {
        self.data_out.clear();
        self.data_in.clear();

        self.busy = false;

        self.output_depth = MdecDepth::MD4Bit;
        self.output_signed = false;
        self.output_msb = false;

        self.current_block = 4;

        self.command = 0;
        self.parameters_remaining = 0;

        self.colour = false;
    }
}