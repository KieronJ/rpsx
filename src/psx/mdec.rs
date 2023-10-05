use std::collections::VecDeque;
use std::mem;

use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

use crate::util;

const MDEC_BLK_CR: usize = 0;
const MDEC_BLK_CB: usize = 1;
const MDEC_BLK_Y: usize = 2;

const MDEC_QT_UV: usize = 0;
const MDEC_QT_Y: usize = 1;

const MDEC_ZAGZIG: [usize; 64] = [
     0,  1,  8, 16,  9,  2,  3, 10,
    17, 24, 32, 25, 18, 11,  4,  5,
    12, 19, 26, 33, 40, 48, 41, 34,
    27, 20, 13,  6,  7, 14, 21, 28,
    35, 42, 49, 56, 57, 50, 43, 36,
    29, 22, 15, 23, 30, 37, 44, 51,
    58, 59, 52, 45, 38, 31, 39, 46,
    53, 60, 61, 54, 47, 55, 62, 63,
];

#[derive(Clone, Copy, Deserialize, Serialize)]
struct QuantTable {
    #[serde(with = "BigArray")]
    data: [u8; 64],
}

impl QuantTable {
    pub fn new() -> QuantTable {
        QuantTable {
            data: [0; 64],
        }
    }
}

#[derive(Clone, Copy, Deserialize, Serialize)]
struct Block {
    #[serde(with = "BigArray")]
    data: [i16; 64],
}

impl Block {
    pub fn new() -> Block {
        Block {
            data: [0; 64],
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct Mdec {
    data_out: VecDeque<u8>,
    data_in: VecDeque<u16>,

    #[serde(with = "BigArray")]
    quant_tables: [QuantTable; 2],

    #[serde(with = "BigArray")]
    scale_table: [i16; 64],

    #[serde(with = "BigArray")]
    blocks: [Block; 3],

    processing_command: bool,
    command: usize,

    current_block: usize,

    words_remaining: u16,
    last_word_received: bool,

    dma0_enable: bool,
    dma1_enable: bool,

    output_depth: u32,
    output_signed: bool,
    output_bit15: bool,

    send_colour: bool,
}

impl Mdec {
    pub fn new() -> Mdec {
        Mdec {
            data_out: VecDeque::new(),
            data_in: VecDeque::new(),

            quant_tables: [QuantTable::new(); 2],
            scale_table: [0; 64],
            blocks: [Block::new(); 3],

            processing_command: false,
            command: 0,

            current_block: 0,

            words_remaining: 0,
            last_word_received: false,

            dma0_enable: false,
            dma1_enable: false,

            output_depth: 0,
            output_signed: false,
            output_bit15: false,

            send_colour: false,
        }
    }

    fn reset(&mut self) {
        self.processing_command = false;

        self.current_block = 4;
        self.words_remaining = 0;
    }

    fn decode_block(&mut self, blk: usize, qt: usize) -> bool {
        let quant = self.quant_tables[qt];

        for i in 0..64 {
            self.blocks[blk].data[i] = 0;
        }

        if self.data_in.is_empty() {
            return false;
        }

        let mut data = self.data_in.pop_front().unwrap();
        let mut k = 0;

        while data == 0xfe00 {
            if self.data_in.is_empty() {
                return false;
            }

            data = self.data_in.pop_front().unwrap();
        }

        let quant_factor = data >> 10;
        let mut dc = (util::sign_extend_u16(data & 0x3ff, 10) as i16) * quant.data[k] as i16;

        loop {
            if quant_factor == 0 {
                dc = (util::sign_extend_u16(data & 0x3ff, 10) as i16) * 2;
            }

            dc = util::clip(dc, -0x400, 0x3ff);

            if quant_factor > 0 {
                self.blocks[blk].data[MDEC_ZAGZIG[k]] = dc;
            } else if quant_factor == 0 {
                self.blocks[blk].data[k] = dc;
            }

            if self.data_in.is_empty() {
                return false;
            }

            data = self.data_in.pop_front().unwrap();

            k += (data >> 10) as usize + 1;

            if k <= 63 {
                dc = ((util::sign_extend_u16(data & 0x3ff, 10) as i16) * (quant.data[k] as i16) * (quant_factor as i16) + 4) >> 3;
                continue;
            }

            break;
        }

        self.idct(blk);

        return true;
    }

    fn idct(&mut self, blk: usize) {
        let src = &mut self.blocks[blk];
        let dst = &mut [0; 64];

        for _ in 0..2 {
            for x in 0..8 {
                for y in 0..8 {
                    let mut sum = 0;

                    for z in 0..8 {
                        sum += (src.data[y + z * 8] as i32) * ((self.scale_table[x + z * 8] as i32) >> 3);
                    }

                    dst[x + y * 8] = ((sum + 0xfff) >> 13) as i16;
                }
            }

            mem::swap(&mut src.data, dst);
        }
    }

    fn yuv_to_rgb(&mut self, output: &mut [u8], xx: usize, yy: usize) {
        for y in 0..8 {
            for x in 0..8 {
                let mut r = self.blocks[MDEC_BLK_CR].data[((x + xx) >> 1) + ((y + yy) >> 1) * 8];
                let mut b = self.blocks[MDEC_BLK_CB].data[((x + xx) >> 1) + ((y + yy) >> 1) * 8];
                let mut g = ((-0.3437 * (b as f32)) + (-0.7143 * (r as f32))) as i16;

                r = (1.402 * (r as f32)) as i16;
                b = (1.772 * (b as f32)) as i16;

                let l = self.blocks[MDEC_BLK_Y].data[x + y * 8];

                r = util::clip(l + r, -128, 127);
                g = util::clip(l + g, -128, 127);
                b = util::clip(l + b, -128, 127);

                if !self.output_signed {
                    r ^= 0x80;
                    g ^= 0x80;
                    b ^= 0x80;
                }

                if self.output_depth == 3 {
                    let r5bit = ((r as u8) >> 3) as u16;
                    let g5bit = ((g as u8) >> 3) as u16;
                    let b5bit = ((b as u8) >> 3) as u16;

                    let mut data = (b5bit << 10) | (g5bit << 5) | r5bit;

                    if self.output_bit15 {
                        data |= 0x8000;
                    }

                    output[0 + ((x + xx) + (y + yy) * 16) * 2] = data as u8;
                    output[1 + ((x + xx) + (y + yy) * 16) * 2] = (data >> 8) as u8;
                } else if self.output_depth == 2 {
                    output[0 + ((x + xx) + (y + yy) * 16) * 3] = r as u8;
                    output[1 + ((x + xx) + (y + yy) * 16) * 3] = g as u8;
                    output[2 + ((x + xx) + (y + yy) * 16) * 3] = b as u8;
                }
            }
        }
    }

    fn process_command(&mut self, value: u32) {
        self.data_in.push_back(value as u16);
        self.data_in.push_back((value >> 16) as u16);
        self.words_remaining -= 1;

        let mut output = [0; 768];

        if self.words_remaining == 0 {
            match self.command {
                1 => {
                    let mut finished;

                    while self.data_in.len() != 0 {
                        match self.current_block {
                            0 => {
                                finished = self.decode_block(MDEC_BLK_Y, MDEC_QT_Y);
                                self.yuv_to_rgb(&mut output, 0, 0);
                            },
                            1 => {
                                finished =  self.decode_block(MDEC_BLK_Y, MDEC_QT_Y);
                                self.yuv_to_rgb(&mut output, 8, 0);
                            },
                            2 => {
                                finished = self.decode_block(MDEC_BLK_Y, MDEC_QT_Y);
                                self.yuv_to_rgb(&mut output, 0, 8);
                            },
                            3 => {
                                finished = self.decode_block(MDEC_BLK_Y, MDEC_QT_Y);
                                self.yuv_to_rgb(&mut output, 8, 8);

                                if self.output_depth == 2 {
                                    for i in 0..768 {
                                        self.data_out.push_back(output[i]);
                                    }
                                } else if self.output_depth == 3 {
                                    for i in 0..512 {
                                        self.data_out.push_back(output[i]);
                                    }
                                }
                            },
                            4 => finished = self.decode_block(MDEC_BLK_CR, MDEC_QT_UV),
                            5 => finished = self.decode_block(MDEC_BLK_CB, MDEC_QT_UV),
                            _ => unreachable!(),
                        };

                        if finished {
                            self.current_block += 1;

                            if self.current_block >= 6 {
                                self.current_block = 0;
                            }
                        }
                    }
                }
                2 => {
                    for i in 0..32 {
                        let half = self.data_in.pop_front().unwrap();
                        self.quant_tables[MDEC_QT_Y].data[i * 2] = half as u8;
                        self.quant_tables[MDEC_QT_Y].data[i * 2 + 1] = (half >> 8) as u8;
                    }

                    if self.send_colour {
                        for i in 0..32 {
                            let half = self.data_in.pop_front().unwrap();
                            self.quant_tables[MDEC_QT_UV].data[i * 2] = half as u8;
                            self.quant_tables[MDEC_QT_UV].data[i * 2 + 1] = (half >> 8) as u8;
                        }
                    }
                }
                3 => {
                    for i in 0..64 {
                        let half = self.data_in.pop_front().unwrap();
                        self.scale_table[i] = half as i16;
                    }
                }
                _ => println!("[MDEC] [ERROR] Unknown command: {}", self.command),
            }

            self.processing_command = false;
            self.last_word_received = true;
        }
    }

    pub fn read_data(&mut self) -> u32 {
        let b0 = self.data_out.pop_front().unwrap() as u32;
        let b1 = self.data_out.pop_front().unwrap() as u32;
        let b2 = self.data_out.pop_front().unwrap() as u32;
        let b3 = self.data_out.pop_front().unwrap() as u32;

        b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
    }

    pub fn write_command(&mut self, value: u32) {
        if self.processing_command {
            self.process_command(value);
            return;
        }

        self.command = (value >> 29) as usize;
        self.processing_command = true;
        self.last_word_received = false;

        match self.command {
            0 => {
                self.words_remaining = 0;
                self.processing_command = false;
                self.last_word_received = true;
            }
            1 => {
                self.words_remaining = value as u16;
                self.output_depth = (value & 0x18000000) >> 27;
                self.output_signed = (value & 0x4000000) != 0;
                self.output_bit15 = (value & 0x2000000) != 0;
            }
            2 => {
                self.send_colour = (value & 0x1) != 0;
                self.words_remaining = match self.send_colour {
                    false => 16,
                    true => 32,
                };
            }
            3 => {
                self.words_remaining = 32;
            }
            _ => println!("[MDEC] [ERROR] Unknown command: {}", self.command),
        };
    }

    pub fn read_status(&self) -> u32 {
        let mut status = 0;

        status |= (self.data_out.is_empty() as u32) << 31;
        status |= ((!self.data_in.is_empty()) as u32) << 30;
        status |= (self.processing_command as u32) << 29;
        status |= self.output_depth << 25;
        status |= (self.output_signed as u32) << 24;
        status |= (self.output_bit15 as u32) << 23;
        status |= (self.current_block as u32) << 16;

        status |= (self.words_remaining - 1) as u32;

        status
    }

    pub fn write_control(&mut self, value: u32) {
        if (value & 0x80000000) != 0 {
            self.reset()
        }

        self.dma0_enable = (value & 0x40000000) != 0;
        self.dma1_enable = (value & 0x20000000) != 0;
    }
}
