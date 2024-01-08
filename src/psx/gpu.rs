use std::cmp;
use std::fs::File;
use std::io::Write;

use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

use byteorder::{ByteOrder, LittleEndian};

use crate::util;

use super::intc::{Intc, Interrupt};
use super::rasteriser::{Colour, Vector2i, Vector3i};
use super::timers::Timers;

// TODO: selectable dithering

#[allow(dead_code)]
pub const DITHER_TABLE: [i32; 16] = [-4, 0, -3, 1, 2, -2, 3, -1, -3, 1, -4, 0, 3, -1, 2, -2];

pub const CMD_SIZE: [usize; 256] = [
    1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    4, 4, 4, 4, 7, 7, 7, 7, 5, 5, 5, 5, 9, 9, 9, 9, 6, 6, 6, 6, 9, 9, 9, 9, 8, 8, 8, 8, 12, 12, 12,
    12, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
    4, 4, 3, 1, 3, 1, 4, 4, 4, 4, 2, 1, 2, 1, 3, 3, 3, 3, 2, 1, 2, 1, 3, 3, 3, 3, 2, 1, 2, 1, 3, 3,
    3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
    4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
    3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
    3, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1,
];

#[derive(Deserialize, Serialize)]
struct Transfer {
    x: u32,
    y: u32,
    w: u32,
    h: u32,

    rx: u32,
    ry: u32,

    active: bool,
}

impl Transfer {
    pub fn new() -> Transfer {
        Transfer {
            x: 0,
            y: 0,
            w: 0,
            h: 0,

            rx: 0,
            ry: 0,

            active: false,
        }
    }
}

#[derive(Clone, Copy, Deserialize, Serialize)]
enum DmaDirection {
    Off,
    Fifo,
    CpuToGp0,
    GpureadToCpu,
}

#[derive(Clone, Copy, Deserialize, PartialEq, Serialize)]
enum TexturePageColours {
    TP4Bit,
    TP8Bit,
    TP15Bit,
    Reserved,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
enum SemiTransparency {
    Half,
    Add,
    Subtract,
    AddQuarter,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
pub struct Texpage {
    flip_y: bool,
    flip_x: bool,
    texture_disable: bool,
    display_area_enable: bool,
    dithering_enable: bool,
    colour_depth: TexturePageColours,
    semi_transparency: SemiTransparency,
    y_base: u32,
    x_base: u32,
}

impl Texpage {
    pub fn new() -> Texpage {
        Texpage {
            flip_y: false,
            flip_x: false,
            texture_disable: false,
            display_area_enable: false,
            dithering_enable: false,
            colour_depth: TexturePageColours::TP4Bit,
            semi_transparency: SemiTransparency::Half,
            y_base: 0,
            x_base: 0,
        }
    }

    pub fn from_u32(value: u32) -> Texpage {
        let texpage = value >> 16;

        Texpage {
            flip_y: (texpage & 0x2000) != 0,
            flip_x: (texpage & 0x1000) != 0,
            texture_disable: (texpage & 0x800) != 0,
            display_area_enable: (texpage & 0x400) != 0,
            dithering_enable: (texpage & 0x200) != 0,
            colour_depth: match (texpage & 0x180) >> 7 {
                0 => TexturePageColours::TP4Bit,
                1 => TexturePageColours::TP8Bit,
                2 => TexturePageColours::TP15Bit,
                3 => TexturePageColours::Reserved,
                _ => unreachable!(),
            },
            semi_transparency: match (texpage & 0x60) >> 5 {
                0 => SemiTransparency::Half,
                1 => SemiTransparency::Add,
                2 => SemiTransparency::Subtract,
                3 => SemiTransparency::AddQuarter,
                _ => unreachable!(),
            },
            y_base: (texpage & 0x10) * 16,
            x_base: (texpage & 0xf) * 64,
        }
    }
}

#[derive(Clone, Copy, Deserialize, Serialize)]
struct CacheEntry {
    tag: isize,
    data: [u8; 8],
}

impl CacheEntry {
    pub fn new() -> CacheEntry {
        CacheEntry {
            tag: -1,
            data: [0; 8],
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct Gpu {
    vram: Box<[u8]>,

    #[serde(with = "BigArray")]
    texture_cache: [CacheEntry; 256],

    #[serde(with = "BigArray")]
    clut_cache: [u16; 256],
    clut_cache_tag: isize,

    scanline: usize,
    video_cycle: usize,
    lines: usize,

    dotclock_cycle: usize,

    gpuread: u32,

    command_buffer: [u32; 16],
    command_buffer_index: usize,

    command_words_remaining: usize,

    cpu_to_gpu_transfer: Transfer,
    gpu_to_cpu_transfer: Transfer,

    interlace_line: bool,
    dma_direction: DmaDirection,

    dma_ready: bool,
    vram_ready: bool,
    cmd_ready: bool,

    irq: bool,

    display_disable: bool,
    vertical_interlace: bool,
    interlace_field: bool,
    colour_depth: bool,
    video_mode: bool,

    vres: u32,
    hres: u32,

    reverse: bool,

    skip_masked_pixels: bool,
    set_mask_bit: bool,

    texpage: Texpage,

    command_tpx: u32,
    command_tpy: u32,
    command_depth: TexturePageColours,
    command_clut_x: i32,
    command_clut_y: i32,

    rectangle: bool,
    line: bool,
    polyline: bool,
    polyline_coord: Vector2i,
    polyline_colour: Colour,
    polyline_remaining: usize,

    shaded: bool,
    semi_tranparent: bool,

    drawing_begin: u32,
    drawing_x_begin: i32,
    drawing_y_begin: i32,

    drawing_end: u32,
    drawing_x_end: i32,
    drawing_y_end: i32,

    drawing_offset: u32,
    drawing_x_offset: i32,
    drawing_y_offset: i32,

    texture_window: u32,
    texture_window_mask_x: u32,
    texture_window_mask_y: u32,
    texture_window_offset_x: u32,
    texture_window_offset_y: u32,

    display_area_x: u32,
    display_area_y: u32,

    horizontal_display_start: u32,
    horizontal_display_end: u32,

    vertical_display_start: u32,
    vertical_display_end: u32,

    frame_complete: bool,
}

impl Gpu {
    pub fn new() -> Gpu {
        Gpu {
            vram: vec![0; 0x100000].into_boxed_slice(),
            texture_cache: [CacheEntry::new(); 256],
            clut_cache: [0; 256],
            clut_cache_tag: -1,

            scanline: 0,
            video_cycle: 0,
            lines: 263,

            dotclock_cycle: 0,

            gpuread: 0,

            command_buffer: [0; 16],
            command_buffer_index: 0,

            command_words_remaining: 0,

            cpu_to_gpu_transfer: Transfer::new(),
            gpu_to_cpu_transfer: Transfer::new(),

            interlace_line: false,
            dma_direction: DmaDirection::Off,

            dma_ready: true,
            vram_ready: true,
            cmd_ready: true,

            irq: false,

            display_disable: false,
            vertical_interlace: false,
            interlace_field: false,
            colour_depth: false,
            video_mode: false,

            vres: 240,
            hres: 320,

            reverse: false,

            skip_masked_pixels: false,
            set_mask_bit: false,

            texpage: Texpage::new(),

            command_tpx: 0,
            command_tpy: 0,
            command_depth: TexturePageColours::TP4Bit,
            command_clut_x: 0,
            command_clut_y: 0,

            rectangle: false,
            line: false,
            polyline: false,
            polyline_coord: Vector2i::new(0, 0),
            polyline_colour: Colour::from_u32(0),
            polyline_remaining: 0,

            shaded: false,
            semi_tranparent: false,

            drawing_begin: 0,
            drawing_x_begin: 0,
            drawing_y_begin: 0,

            drawing_end: 0,
            drawing_x_end: 0,
            drawing_y_end: 0,

            drawing_offset: 0,
            drawing_x_offset: 0,
            drawing_y_offset: 0,

            texture_window: 0,
            texture_window_mask_x: 0,
            texture_window_mask_y: 0,
            texture_window_offset_x: 0,
            texture_window_offset_y: 0,

            display_area_x: 0,
            display_area_y: 0,

            horizontal_display_start: 512,
            horizontal_display_end: 3072,

            vertical_display_start: 16,
            vertical_display_end: 256,

            frame_complete: false,
        }
    }

    pub fn tick(&mut self, intc: &mut Intc, timers: &mut Timers, clocks: usize) {
        let cycles = self.horizontal_length();
        let dotclock = self.get_dotclock() as usize;

        let old_hblank = self.in_hblank();
        let old_vblank = self.in_vblank();

        self.video_cycle += clocks;
        self.dotclock_cycle += clocks;

        timers.tick_dotclock(intc, self.dotclock_cycle / dotclock);

        self.dotclock_cycle %= dotclock;

        if self.video_cycle >= cycles {
            self.video_cycle -= cycles;

            timers.tick_hblank(intc);

            if self.vres == 240 && self.vertical_interlace {
                self.interlace_line = !self.interlace_line;
            }

            self.scanline += 1;

            if self.scanline == (self.lines - 20) {
                self.frame_complete = true;
                intc.assert_irq(Interrupt::Vblank);
            }

            if self.scanline == self.lines {
                if self.lines == 263 {
                    self.lines = 262;
                } else {
                    self.lines = 263;
                }

                self.scanline = 0;

                if self.vres == 480 && self.vertical_interlace {
                    self.interlace_line = !self.interlace_line;
                }

                self.interlace_field = !self.interlace_field;
            }
        }

        if self.in_hblank() {
            if !old_hblank {
                timers.set_hblank(true);
            }
        } else {
            if old_hblank {
                timers.set_hblank(false);
            }
        }

        if self.in_vblank() {
            if !old_vblank {
                timers.set_vblank(true);
            }
        } else {
            if old_vblank {
                timers.set_vblank(false);
            }
        }

        if self.irq {
            intc.assert_irq(Interrupt::Gpu);
        }
    }

    fn horizontal_length(&self) -> usize {
        match self.video_mode {
            true => 3406,
            false => 3413,
        }
    }

    pub fn in_hblank(&self) -> bool {
        self.video_cycle < self.horizontal_display_start as usize
            || self.video_cycle >= self.horizontal_display_end as usize
    }

    pub fn in_vblank(&self) -> bool {
        self.scanline >= (self.lines - 20)
    }

    pub fn get_dotclock(&self) -> u32 {
        match self.hres {
            320 => 8,
            640 => 4,
            256 => 10,
            512 => 5,
            368 => 7,
            _ => unreachable!(),
        }
    }

    pub fn get_display_origin(&self) -> (u32, u32) {
        (self.display_area_x, self.display_area_y)
    }

    pub fn get_display_size(&self) -> (u32, u32) {
        let xstart = self.horizontal_display_start;
        let xend = self.horizontal_display_end;
        let dotclock = self.get_dotclock();

        let ystart = self.vertical_display_start;
        let yend = self.vertical_display_end;

        let xdiff;

        if xstart <= xend {
            xdiff = xend - xstart;
        } else {
            xdiff = 50;
        }

        let x = ((xdiff / dotclock) + 2) & !0x3;
        let mut y = yend - ystart;

        if self.vertical_interlace {
            y *= 2;
        }

        (x, y)
    }

    pub fn get_framebuffer(&self,
                           framebuffer: &mut [u8],
                           draw_full_vram: bool) {
        let (xs, ys) = if draw_full_vram {
            (0, 0)
        } else {
            let (mut x, mut y) = self.get_display_origin();

            // Adjust start based on CRTC registers
            x += (self.horizontal_display_start - 608) / self.get_dotclock();
            y += (self.vertical_display_start - 16) * 2;

            (x, y)
        };

        let (w, h) = if draw_full_vram {
            (0, 0)
        } else {
            self.get_display_size()
        };

        let mut framebuffer_address = 0;

        for y in ys..ys + h {
            for x in xs..xs + w {
                let address = match !draw_full_vram && self.colour_depth {
                    true => Gpu::vram_address_24bit(x, y),
                    false => Gpu::vram_address(x, y),
                };

                let col;

                if !draw_full_vram && self.colour_depth {
                    let r = self.vram[address];
                    let g = self.vram[address + 1];
                    let b = self.vram[address + 2];
                    col = Colour::new(r, g, b, false);
                } else {
                    let colour = LittleEndian::read_u16(&self.vram[address..]);
                    col = Colour::from_u16(colour);
                }

                framebuffer[framebuffer_address] = col.r;
                framebuffer[framebuffer_address + 1] = col.g;
                framebuffer[framebuffer_address + 2] = col.b;
                framebuffer_address += 3;
            }
        }
    }

    pub fn dump_vram(&self) {
        let mut file = File::create("vram.bin").unwrap();
        file.write_all(&self.vram).unwrap();
    }

    pub fn frame_complete(&mut self) -> bool {
        if self.frame_complete {
            self.frame_complete = false;
            return true;
        }

        false
    }

    pub fn gpuread(&mut self) -> u32 {
        if self.gpu_to_cpu_transfer.active {
            let lo = self.vram_read_transfer() as u32;
            let hi = self.vram_read_transfer() as u32;

            return (hi << 16) | lo;
        }

        self.gpuread
    }

    pub fn gpustat(&mut self) -> u32 {
        let mut value = 0;

        let interlace_line = match self.in_vblank() {
            true => false,
            false => self.interlace_line,
        };

        value |= (interlace_line as u32) << 31;
        value |= (self.dma_direction as u32) << 29;
        value |= (self.dma_ready as u32) << 28;
        value |= (self.vram_ready as u32) << 27;
        value |= (self.cmd_ready as u32) << 26;
        value |= match self.dma_direction {
            DmaDirection::Off => 0,
            DmaDirection::Fifo => 1,
            DmaDirection::CpuToGp0 => self.dma_ready as u32,
            DmaDirection::GpureadToCpu => self.vram_ready as u32,
        } << 25;
        value |= (self.irq as u32) << 24;
        value |= (self.display_disable as u32) << 23;
        value |= (self.vertical_interlace as u32) << 22;
        value |= (self.colour_depth as u32) << 21;
        value |= (self.video_mode as u32) << 20;
        value |= match self.vres {
            480 => 1 << 19,
            240 => 0,
            _ => unreachable!(),
        };
        value |= match self.hres {
            256 => 0x00,
            320 => 0x02,
            512 => 0x04,
            640 => 0x06,
            368 => 0x01,
            _ => unreachable!(),
        } << 16;
        value |= (self.texpage.texture_disable as u32) << 15;
        value |= (self.reverse as u32) << 14;
        value |= match self.vertical_interlace {
            true => self.interlace_field as u32,
            false => 1,
        } << 13;
        value |= (self.skip_masked_pixels as u32) << 12;
        value |= (self.set_mask_bit as u32) << 11;
        value |= (self.texpage.display_area_enable as u32) << 10;
        value |= (self.texpage.dithering_enable as u32) << 9;
        value |= (self.texpage.colour_depth as u32) << 7;
        value |= (self.texpage.semi_transparency as u32) << 5;
        value |= self.texpage.y_base / 16;
        value |= self.texpage.x_base / 64;

        value
    }

    pub fn gp0_write(&mut self, word: u32) {
        if self.cpu_to_gpu_transfer.active {
            self.vram_write_transfer(word as u16);

            if self.cpu_to_gpu_transfer.active {
                self.vram_write_transfer((word >> 16) as u16);
            }

            return;
        }

        if self.polyline {
            if (word & 0x50005000) == 0x50005000 {
                self.polyline = false;
                return;
            }

            self.command_buffer[self.command_buffer_index] = word;
            self.command_buffer_index += 1;

            self.polyline_remaining -= 1;

            if self.polyline_remaining == 0 {
                let coords = [self.polyline_coord; 2];
                let mut colours = [self.polyline_colour; 2];

                if self.shaded {
                    colours[1] = Colour::from_u32(self.command_buffer[0]);
                }

                let _coord2 = self.command_buffer[1];
                //coords[1] = self.get_coord(coord2);

                //self.rasterise_line(coords, colours);

                self.polyline_coord = coords[1];
                self.polyline_colour = colours[1];

                self.polyline_remaining = match self.shaded {
                    false => 1,
                    true => 2,
                };

                self.command_buffer_index = 0;
            }

            return;
        }

        self.push_gp0_command(word);
    }

    fn vram_address(x: u32, y: u32) -> usize {
        2 * ((x & 0x3ff) + 1024 * (y & 0x1ff)) as usize
    }

    fn vram_address_24bit(x: u32, y: u32) -> usize {
        (3 * (x & 0x3ff) + 2048 * (y & 0x1ff)) as usize
    }

    fn vram_read_transfer(&mut self) -> u16 {
        let x = self.gpu_to_cpu_transfer.x + self.gpu_to_cpu_transfer.rx;
        let y = self.gpu_to_cpu_transfer.y + self.gpu_to_cpu_transfer.ry;

        self.gpu_to_cpu_transfer.rx += 1;

        if self.gpu_to_cpu_transfer.rx == self.gpu_to_cpu_transfer.w {
            self.gpu_to_cpu_transfer.rx = 0;

            self.gpu_to_cpu_transfer.ry += 1;

            if self.gpu_to_cpu_transfer.ry == self.gpu_to_cpu_transfer.h {
                self.gpu_to_cpu_transfer.ry = 0;

                self.gpu_to_cpu_transfer.active = false;
            }
        }

        let destination_address = Gpu::vram_address(x, y);
        LittleEndian::read_u16(&self.vram[destination_address..])
    }

    fn vram_write_transfer(&mut self, mut data: u16) {
        let x = self.cpu_to_gpu_transfer.x + self.cpu_to_gpu_transfer.rx;
        let y = self.cpu_to_gpu_transfer.y + self.cpu_to_gpu_transfer.ry;

        let destination_address = Gpu::vram_address(x & 0x3ff, y & 0x1ff);

        self.cpu_to_gpu_transfer.rx += 1;

        if self.cpu_to_gpu_transfer.rx == self.cpu_to_gpu_transfer.w {
            self.cpu_to_gpu_transfer.rx = 0;

            self.cpu_to_gpu_transfer.ry += 1;

            if self.cpu_to_gpu_transfer.ry == self.cpu_to_gpu_transfer.h {
                self.cpu_to_gpu_transfer.ry = 0;
                self.cpu_to_gpu_transfer.active = false;
            }
        }

        if self.skip_masked_pixels {
            let prev = LittleEndian::read_u16(&self.vram[destination_address..]);

            if (prev & 0x8000) != 0 {
                return;
            }
        }

        if self.set_mask_bit {
            data |= 0x8000;
        }

        LittleEndian::write_u16(&mut self.vram[destination_address..], data);
    }

    fn push_gp0_command(&mut self, command_word: u32) {
        if self.command_buffer_index < 16 {
            self.command_buffer[self.command_buffer_index] = command_word;
            self.command_buffer_index += 1;
        }

        if self.command_buffer_index >= 16 {
            self.cmd_ready = false;
        }

        if self.command_words_remaining == 0 {
            let command = (command_word >> 24) as usize;
            self.command_words_remaining = CMD_SIZE[command];
        }

        if self.command_words_remaining == 1 {
            self.execute_gp0_command();
            self.command_buffer_index = 0;
        }

        self.command_words_remaining -= 1;
    }

    fn execute_gp0_command(&mut self) {
        let command_word = self.command_buffer[0];
        let command = command_word >> 24;

        match command {
            0x00 => {} // NOP
            0x01 => self.invalidate_cache(),
            0x02 => {
                let destination = self.command_buffer[1];
                let size = self.command_buffer[2];

                let colour = Colour::from_u32(command_word);
                let pixel = colour.to_u16();

                let x_start = destination & 0x3f0;
                let y_start = (destination >> 16) & 0x3ff;

                let w = ((size & 0x3ff) + 0xf) & !0xf;
                let h = (size >> 16) & 0x1ff;

                for y in 0..h {
                    for x in 0..w {
                        let destination_address =
                            Gpu::vram_address((x_start + x) & 0x3ff, (y_start + y) & 0x1ff);
                        LittleEndian::write_u16(&mut self.vram[destination_address..], pixel);
                    }
                }
            }
            0x03..=0x1e => {} // NOP
            0x1f => self.irq = true,
            0x20..=0x3f => self.draw_polygon(),
            0x40..=0x5f => self.draw_line(),
            0x60..=0x7f => self.draw_rectangle(),
            0x80..=0x9f => {
                let src = self.command_buffer[1];
                let dest = self.command_buffer[2];
                let size = self.command_buffer[3];

                let src_x = src & 0x3ff;
                let src_y = (src >> 16) & 0x3ff;
                let dest_x = dest & 0x3ff;
                let dest_y = (dest >> 16) & 0x3ff;
                let mut w = size & 0x3ff;
                let mut h = (size >> 16) & 0x1ff;

                if w == 0 { w = 0x400; }
                if h == 0 { h = 0x200; }

                for y in 0..h {
                    for x in 0..w {
                        let src_address =
                            Gpu::vram_address((src_x + x) & 0x3ff, (src_y + y) & 0x1ff);
                        let dest_address =
                            Gpu::vram_address((dest_x + x) & 0x3ff, (dest_y + y) & 0x1ff);

                        let mut data = LittleEndian::read_u16(&self.vram[src_address..]);

                        if self.skip_masked_pixels {
                            let prev = LittleEndian::read_u16(&self.vram[dest_address..]);

                            if (prev & 0x8000) != 0 {
                                continue;
                            }
                        }

                        if self.set_mask_bit {
                            data |= 0x8000;
                        }

                        LittleEndian::write_u16(&mut self.vram[dest_address..], data);
                    }
                }
            }
            0xa0..=0xbf => {
                let destination = self.command_buffer[1];
                let size = self.command_buffer[2];

                let x = destination & 0x3ff;
                let y = (destination >> 16) & 0x3ff;
                let w = size & 0x3ff;
                let h = (size >> 16) & 0x1ff;

                self.cpu_to_gpu_transfer.x = x;
                self.cpu_to_gpu_transfer.y = y;
                self.cpu_to_gpu_transfer.w = w;
                self.cpu_to_gpu_transfer.h = h;

                if self.cpu_to_gpu_transfer.w == 0 {
                    self.cpu_to_gpu_transfer.w = 0x400;
                }

                if self.cpu_to_gpu_transfer.h == 0 {
                    self.cpu_to_gpu_transfer.h = 0x200;
                }

                self.cpu_to_gpu_transfer.rx = 0;
                self.cpu_to_gpu_transfer.ry = 0;

                self.cpu_to_gpu_transfer.active = true;
            }
            0xc0..=0xdf => {
                let destination = self.command_buffer[1];
                let size = self.command_buffer[2];

                let x = destination & 0x3ff;
                let y = (destination >> 16) & 0x1ff;
                let w = size & 0x3ff;
                let h = (size >> 16) & 0x1ff;

                self.gpu_to_cpu_transfer.x = x;
                self.gpu_to_cpu_transfer.y = y;
                self.gpu_to_cpu_transfer.w = w;
                self.gpu_to_cpu_transfer.h = h;

                self.gpu_to_cpu_transfer.rx = 0;
                self.gpu_to_cpu_transfer.ry = 0;

                self.gpu_to_cpu_transfer.active = true;
            }
            0xe0 => {} // NOP
            0xe1 => {
                self.texpage.flip_y = (command_word & 0x2000) != 0;
                self.texpage.flip_x = (command_word & 0x1000) != 0;
                self.texpage.texture_disable = (command_word & 0x800) != 0;
                self.texpage.display_area_enable = (command_word & 0x400) != 0;
                self.texpage.dithering_enable = (command_word & 0x200) != 0;

                self.texpage.colour_depth = match (command_word & 0x180) >> 7 {
                    0 => TexturePageColours::TP4Bit,
                    1 => TexturePageColours::TP8Bit,
                    2 => TexturePageColours::TP15Bit,
                    3 => TexturePageColours::Reserved,
                    _ => unreachable!(),
                };

                self.texpage.semi_transparency = match (command_word & 0x60) >> 5 {
                    0 => SemiTransparency::Half,
                    1 => SemiTransparency::Add,
                    2 => SemiTransparency::Subtract,
                    3 => SemiTransparency::AddQuarter,
                    _ => unreachable!(),
                };

                self.texpage.y_base = (command_word & 0x10) * 16;
                self.texpage.x_base = (command_word & 0xf) * 64;
            }
            0xe2 => {
                self.texture_window = command_word & 0xf_ffff;
                self.texture_window_offset_y = ((command_word & 0xf_8000) >> 15) * 8;
                self.texture_window_offset_x = ((command_word & 0x7c00) >> 10) * 8;
                self.texture_window_mask_y = ((command_word & 0x3e0) >> 5) * 8;
                self.texture_window_mask_x = (command_word & 0x1f) * 8;
            }
            0xe3 => {
                let x = command_word & 0x3ff;
                let y = (command_word & 0x7_fc00) >> 10;

                self.drawing_begin = command_word & 0x7_ffff;
                self.drawing_y_begin = y as i32;
                self.drawing_x_begin = x as i32;
            }
            0xe4 => {
                let x = command_word & 0x3ff;
                let y = (command_word & 0x7_fc00) >> 10;

                self.drawing_end = command_word & 0x7_ffff;
                self.drawing_y_end = y as i32;
                self.drawing_x_end = x as i32;
            }
            0xe5 => {
                self.drawing_offset = command_word & 0x3f_ffff;

                let dyo = (command_word >> 11) & 0x7ff;
                let dxo = command_word & 0x7ff;

                self.drawing_y_offset = util::sign_extend_i32(dyo as i32, 11);
                self.drawing_x_offset = util::sign_extend_i32(dxo as i32, 11);
            }
            0xe6 => {
                self.skip_masked_pixels = (command_word & 0x2) != 0;
                self.set_mask_bit = (command_word & 0x1) != 0;
            }
            0xe7..=0xff => {} // NOP
            _ => panic!("[GPU] [ERROR] Unknown command GP0({:02x})", command),
        }
    }

    pub fn execute_gp1_command(&mut self, command_word: u32) {
        let command = command_word >> 24;

        match command {
            0x00 => {
                self.execute_gp1_command(0x0100_0000);
                self.execute_gp1_command(0x0200_0000);
                self.execute_gp1_command(0x0300_0001);
                self.execute_gp1_command(0x0400_0000);
                self.execute_gp1_command(0x0500_0000);
                self.execute_gp1_command(0x0600_0000);
                self.execute_gp1_command(0x0700_0000);
                self.execute_gp1_command(0x0800_0000);

                self.texpage.flip_y = false;
                self.texpage.flip_x = false;
                self.texpage.texture_disable = false;
                self.texpage.display_area_enable = false;
                self.texpage.dithering_enable = false;
                self.texpage.colour_depth = TexturePageColours::TP4Bit;
                self.texpage.semi_transparency = SemiTransparency::Half;
                self.texpage.y_base = 0;
                self.texpage.x_base = 0;

                self.texture_window_offset_y = 0;
                self.texture_window_offset_x = 0;
                self.texture_window_mask_y = 0;
                self.texture_window_mask_x = 0;

                self.drawing_y_begin = 0;
                self.drawing_x_begin = 0;

                self.drawing_y_end = 0;
                self.drawing_x_end = 0;

                self.drawing_y_offset = 0;
                self.drawing_x_offset = 0;

                self.horizontal_display_start = 512;
                self.horizontal_display_end = 3072;

                self.vertical_display_start = 16;
                self.vertical_display_end = 256;

                self.video_mode = false;
                self.hres = 320;
                self.vres = 240;
                self.vertical_interlace = false;
                self.interlace_field = false;

                self.skip_masked_pixels = false;
                self.set_mask_bit = false;
            }
            0x01 => self.command_buffer_index = 0,
            0x02 => self.irq = false,
            0x03 => self.display_disable = (command_word & 0x1) != 0,
            0x04 => {
                self.dma_direction = match command_word & 0x3 {
                    0 => DmaDirection::Off,
                    1 => DmaDirection::Fifo,
                    2 => DmaDirection::CpuToGp0,
                    3 => DmaDirection::GpureadToCpu,
                    _ => unreachable!(),
                };
            }
            0x05 => {
                self.display_area_y = (command_word & 0x7_fc00) >> 10;
                self.display_area_x = command_word & 0x3ff;
            }
            0x06 => {
                self.horizontal_display_end = (command_word & 0xff_f000) >> 12;
                self.horizontal_display_start = command_word & 0xfff;
            }
            0x07 => {
                self.vertical_display_end = (command_word & 0xf_fc00) >> 10;
                self.vertical_display_start = command_word & 0x3ff;
            }
            0x08 => {
                self.reverse = (command_word & 0x80) != 0;
                self.vertical_interlace = (command_word & 0x20) != 0;
                self.colour_depth = (command_word & 0x10) != 0;
                self.video_mode = (command_word & 0x8) != 0;

                self.vres = match (self.vertical_interlace, (command_word & 0x4) != 0) {
                    (true, true) => 480,
                    _ => 240,
                };

                self.hres = match ((command_word & 0x40) != 0, command_word & 0x3) {
                    (true, _) => 368,
                    (false, 0) => 256,
                    (false, 1) => 320,
                    (false, 2) => 512,
                    (false, 3) => 640,
                    _ => unreachable!(),
                };
            }
            0x09 => (), //New Texture Disable
            0x10..=0x1f => {
                //println!("GPUREAD command {:#x}", command_word);

                match command_word & 0x07 {
                    0x02 => self.gpuread = self.texture_window,
                    0x03 => self.gpuread = self.drawing_begin,
                    0x04 => self.gpuread = self.drawing_end,
                    0x05 => self.gpuread = self.drawing_offset,
                    _ => (),
                };
            }
            0x20 => (), //Arcade Texture Disable
            _ => panic!("[GPU] [ERROR] Unknown command GP1({:02x})", command),
        }
    }

    fn to_coord(&self, value: u32) -> Vector2i {
        let x = util::sign_extend_i32((value & 0xffff) as i32, 11);
        let y = util::sign_extend_i32((value >> 16) as i32, 11);

        let xoffset = self.drawing_x_offset;
        let yoffset = self.drawing_y_offset;

        Vector2i::new(x + xoffset, y + yoffset)
    }

    fn to_texcoord(&self, value: u32) -> Vector2i {
        let x = value & 0xff;
        let y = (value & 0xff00) >> 8;

        Vector2i::new(x as i32, y as i32)
    }

    fn mask_texcoord(&self, mut uv: Vector2i) -> Vector2i {
        let mask_x = self.texture_window_mask_x as i32;
        let mask_y = self.texture_window_mask_y as i32;

        let offset_x = self.texture_window_offset_x as i32;
        let offset_y = self.texture_window_offset_y as i32;

        uv.x = (uv.x & !mask_x) | (offset_x & mask_x);
        uv.y = (uv.y & !mask_y) | (offset_y & mask_y);

        uv
    }

    fn to_clut(value: u32) -> Vector2i {
        let x = ((value >> 16) & 0x3f) << 4;
        let y = ((value >> 16) & 0x7fc0) >> 6;

        Vector2i::new(x as i32, y as i32)
    }

    fn draw_polygon(&mut self) {
        let command = self.command_buffer[0] >> 24;

        let mut vertices = [Vector2i::new(0, 0); 4];
        let mut colours = [Colour::from_u32(self.command_buffer[0]); 4];
        let mut texcoords = [Vector2i::new(0, 0); 4];
        let mut clut = Vector2i::new(0, 0);
        let mut texpage = self.texpage;

        let shaded = (command & 0x10) != 0;
        let points = match (command & 0x8) != 0 {
            true => 4,
            false => 3,
        };
        let textured = (command & 0x4) != 0;
        let transparency = (command & 0x2) != 0;
        let blend = (command & 0x1) == 0;

        let mut pos = 0;

        for i in 0..points {
            if shaded || (i == 0) {
                colours[i] = Colour::from_u32(self.command_buffer[pos]);
                pos += 1;
            }

            vertices[i] = self.to_coord(self.command_buffer[pos]);
            pos += 1;

            if textured {
                texcoords[i] = self.to_texcoord(self.command_buffer[pos]);

                if i == 0 {
                    clut = Gpu::to_clut(self.command_buffer[pos]);
                } else if i == 1 {
                    texpage = Texpage::from_u32(self.command_buffer[pos]);
                }

                pos += 1;
            }
        }

        if textured {
            if (texpage.x_base != self.command_tpx)
               || (texpage.y_base != self.command_tpy)
               || (texpage.colour_depth != self.command_depth)
               || (clut.x != self.command_clut_x)
               || (clut.y != self.command_clut_y) {
                self.invalidate_cache();
            }

            self.command_tpx = texpage.x_base;
            self.command_tpy = texpage.y_base;
            self.command_depth = texpage.colour_depth;
            self.command_clut_x = clut.x;
            self.command_clut_y = clut.y;

            self.texpage = texpage;
        }

        colours[0] = Colour::from_u32(self.command_buffer[0]);
        self.rasterise_triangle(&vertices[0..3],
                                &colours[0..3],
                                &texcoords[0..3],
                                clut,
                                shaded, textured,
                                blend, transparency);

        if points == 4 {
            self.rasterise_triangle(&vertices[1..4],
                                    &colours[1..4],
                                    &texcoords[1..4],
                                    clut,
                                    shaded, textured,
                                    blend, transparency);
        }
    }

    fn draw_line(&mut self) {
        let command = self.command_buffer[0] >> 24;

        let shaded = (command & 0x10) != 0;
        let polyline = (command & 0x8) != 0;
        let _transparency = (command & 0x2) != 0;

        self.polyline = polyline;
        self.polyline_remaining = match shaded {
            false => 1,
            true => 2,
        };
    }

    fn draw_rectangle(&mut self) {
        let command = self.command_buffer[0] >> 24;

        let rect_size = (command & 0x18) >> 3;
        let textured = (command & 0x4) != 0;
        let transparency = (command & 0x2) != 0;
        let blend = (command & 0x1) == 0;

        let colour = Colour::from_u32(self.command_buffer[0]);

        let vertex = self.to_coord(self.command_buffer[1]);

        let texpage = self.texpage;

        let mut texcoord = Vector2i::new(0, 0);
        let mut clut = Vector2i::new(0, 0);

        let mut pos = 2;

        if textured {
            texcoord = self.to_texcoord(self.command_buffer[pos]);
            clut = Gpu::to_clut(self.command_buffer[pos]);

            if (texpage.x_base != self.command_tpx)
               || (texpage.y_base != self.command_tpy)
               || (texpage.colour_depth != self.command_depth)
               || (clut.x != self.command_clut_x)
               || (clut.y != self.command_clut_y) {
                self.invalidate_cache();
            }

            self.command_tpx = texpage.x_base;
            self.command_tpy = texpage.y_base;
            self.command_depth = texpage.colour_depth;
            self.command_clut_x = clut.x;
            self.command_clut_y = clut.y;

            pos += 1;
        }

        let size = match rect_size {
            0 => {
                let tmp = self.command_buffer[pos];
                let x = (tmp & 0x3ff) as i32;
                let y = ((tmp >> 16) & 0x1ff) as i32;

                Vector2i::new(x, y)
            },
            1 => Vector2i::new(1, 1),
            2 => Vector2i::new(8, 8),
            3 => Vector2i::new(16, 16),
            _ => unreachable!(),
        };

        for y in 0..size.y {
            for x in 0..size.x {
                let p = Vector2i::new(vertex.x + x, vertex.y + y);

                if (p.x < self.drawing_x_begin) || (p.x > self.drawing_x_end)
                   || (p.y < self.drawing_y_begin) || (p.y > self.drawing_y_end) {
                    continue;
                }

                let mut output = colour;

                if textured {
                    let mut uv = Vector2i::new(texcoord.x + (x & 0xff), texcoord.y + (y & 0xff));
                    uv = self.mask_texcoord(uv);

                    let (mut texture, skip) = self.get_texture(uv, clut);

                    if skip {
                        continue;
                    }

                    if blend {
                        texture.r = util::clip((texture.r() * colour.r()) >> 7, 0, 255) as u8;
                        texture.g = util::clip((texture.g() * colour.g()) >> 7, 0, 255) as u8;
                        texture.b = util::clip((texture.b() * colour.b()) >> 7, 0, 255) as u8;
                    }

                    output = texture;
                }

                self.render_pixel(p, output, transparency, !textured);
            }
        }
    }

    fn interpolate_colour(area: i32, w: Vector3i,
                          c0: Colour,
                          c1: Colour,
                          c2: Colour) -> Colour {
        let r = (w.x * c0.r() + w.y * c1.r() + w.z * c2.r()) / area;
        let g = (w.x * c0.g() + w.y * c1.g() + w.z * c2.g()) / area;
        let b = (w.x * c0.b() + w.y * c1.b() + w.z * c2.b()) / area;

        Colour::new(r as u8, g as u8, b as u8, false)
    }

    fn interpolate_texcoord(area: i32, w: Vector3i,
                            t0: Vector2i,
                            t1: Vector2i,
                            t2: Vector2i) -> Vector2i {
        let u = (w.x * t0.x + w.y * t1.x + w.z * t2.x) / area;
        let v = (w.x * t0.y + w.y * t1.y + w.z * t2.y) / area;

        Vector2i::new(u, v)
    }

    fn is_top_left(x: i32, y: i32) -> bool {
        (y < 0) || ((x < 0) && (y == 0))
    }

    fn rasterise_triangle(&mut self,
                          vertices: &[Vector2i],
                          colours: &[Colour],
                          texcoords: &[Vector2i],
                          clut: Vector2i,
                          shaded: bool, textured: bool,
                          blend: bool, transparency: bool) {
        let mut v = [vertices[0], vertices[1], vertices[2]];
        let mut c = [colours[0], colours[1], colours[2]];
        let mut t = [texcoords[0], texcoords[1], texcoords[2]];

        let mut area = Vector2i::orient2d(v[0], v[1], v[2]);

        if area < 0 {
            v.swap(1, 2);
            c.swap(1, 2);
            t.swap(1, 2);

            area = -area;
        } else if area == 0 {
            return;
        }

        let mut minx = util::min3(v[0].x, v[1].x, v[2].x);
        let mut miny = util::min3(v[0].y, v[1].y, v[2].y);

        let mut maxx = util::max3(v[0].x, v[1].x, v[2].x);
        let mut maxy = util::max3(v[0].y, v[1].y, v[2].y);

        if (maxx >= 1024 && minx >= 1024) || (maxx < 0 && minx < 0) {
            return;
        }

        if (maxy >= 512 && miny >= 512) || (maxy < 0 && miny < 0) {
            return;
        }

        if (maxx - minx) >= 1024 {
            return;
        }

        if (maxy - miny) >= 512 {
            return;
        }

        minx = cmp::max(minx, self.drawing_x_begin);
        miny = cmp::max(miny, self.drawing_y_begin);

        maxx = cmp::min(maxx, self.drawing_x_end);
        maxy = cmp::min(maxy, self.drawing_y_end);

        let a01 = v[0].y - v[1].y; let b01 = v[1].x - v[0].x;
        let a12 = v[1].y - v[2].y; let b12 = v[2].x - v[1].x;
        let a20 = v[2].y - v[0].y; let b20 = v[0].x - v[2].x;

        let mut p = Vector2i::new(minx, miny);

        let mut w0_row = Vector2i::orient2d(v[1], v[2], p);
        let mut w1_row = Vector2i::orient2d(v[2], v[0], p);
        let mut w2_row = Vector2i::orient2d(v[0], v[1], p);

        let w0_bias = -(Gpu::is_top_left(b12, a12) as i32);
        let w1_bias = -(Gpu::is_top_left(b20, a20) as i32);
        let w2_bias = -(Gpu::is_top_left(b01, a01) as i32);

        let mut colour = c[0];

        while p.y < maxy {
            let mut w0 = w0_row;
            let mut w1 = w1_row;
            let mut w2 = w2_row;

            p.x = minx;

            while p.x < maxx {
                if ((w0 + w0_bias) | (w1 + w1_bias) | (w2 + w2_bias)) >= 0 {
                    let w = Vector3i::new(w0, w1, w2);

                    if shaded {
                        colour = Gpu::interpolate_colour(area, w, c[0], c[1], c[2]);
                    }

                    let mut output = colour;

                    if textured {
                        let mut uv = Gpu::interpolate_texcoord(area, w, t[0], t[1], t[2]);
                        uv = self.mask_texcoord(uv);

                        let (mut texture, skip) = self.get_texture(uv, clut);

                        if skip {
                            w0 += a12;
                            w1 += a20;
                            w2 += a01;

                            p.x += 1;
                            continue;
                        }

                        if blend {
                            texture.r = util::clip((texture.r() * colour.r()) >> 7, 0, 255) as u8;
                            texture.g = util::clip((texture.g() * colour.g()) >> 7, 0, 255) as u8;
                            texture.b = util::clip((texture.b() * colour.b()) >> 7, 0, 255) as u8;
                        }

                        output = texture;
                    }

                    self.render_pixel(p, output, transparency, !textured);
                }

                w0 += a12;
                w1 += a20;
                w2 += a01;

                p.x += 1;
            }

            w0_row += b12;
            w1_row += b20;
            w2_row += b01;

            p.y += 1;
        }
    }

    fn render_pixel(&mut self, p: Vector2i, c: Colour,
                    transparency: bool, force_blend: bool) {
        let address = Gpu::vram_address(p.x as u32, p.y as u32);
        let back = Colour::from_u16(LittleEndian::read_u16(&self.vram[address..]));

        let mut colour = c;

        if self.skip_masked_pixels && back.a {
            return;
        }

        if (force_blend || c.a) && transparency {
            let r; let g; let b;

            match self.texpage.semi_transparency {
                SemiTransparency::Half => {
                    r = (back.r() + c.r()) / 2;
                    g = (back.g() + c.g()) / 2;
                    b = (back.b() + c.b()) / 2;
                }
                SemiTransparency::Add => {
                    r = back.r() + c.r();
                    g = back.g() + c.g();
                    b = back.b() + c.b();
                }
                SemiTransparency::Subtract => {
                    r = back.r() - c.r();
                    g = back.g() - c.g();
                    b = back.b() - c.b();
                }
                SemiTransparency::AddQuarter => {
                    r = back.r() + c.r() / 4;
                    g = back.g() + c.g() / 4;
                    b = back.b() + c.b() / 4;
                }
            };

            colour.r = util::clip(r, 0, 255) as u8;
            colour.g = util::clip(g, 0, 255) as u8;
            colour.b = util::clip(b, 0, 255) as u8;
        }

        if self.set_mask_bit {
            colour.a = true;
        }

        LittleEndian::write_u16(&mut self.vram[address..], colour.to_u16());
    }

    fn get_texture(&mut self, uv: Vector2i, clut: Vector2i) -> (Colour, bool) {
        use self::TexturePageColours::*;

        match self.texpage.colour_depth {
            TP4Bit => self.read_clut_4bit(uv, clut),
            TP8Bit => self.read_clut_8bit(uv, clut),
            TP15Bit | Reserved => self.read_texture(uv),
        }
    }

    fn invalidate_cache(&mut self) {
        for i in 0..256 {
            self.texture_cache[i].tag = -1;
        }

        self.clut_cache_tag = -1;
    }

    fn read_clut_4bit(&mut self, uv: Vector2i, clut: Vector2i) -> (Colour, bool) {
        let address_x = 2 * self.texpage.x_base + ((uv.x / 2) & 0xff) as u32;
        let address_y = self.texpage.y_base + (uv.y & 0xff) as u32;
        let texture_address = (address_x + 2048 * address_y) as usize;

        let block = (((uv.y >> 6) << 2) + (uv.x >> 6)) as isize;
        let entry = (((uv.y & 0x3f) << 2) + ((uv.x & 0x3f) >> 4)) as usize;

        let index = ((uv.x >> 1) & 0x7) as usize;

        let centry = &mut self.texture_cache[entry];

        if centry.tag != block {
            for i in 0..8 {
                centry.data[i] = self.vram[(texture_address & !0x7) + i];
            }

            centry.tag = block;
        }

        let mut clut_entry = centry.data[index] as usize;

        if (uv.x & 0x1) != 0 {
            clut_entry >>= 4;
        } else {
            clut_entry &= 0xf;
        }

        let clut_address = (2 * clut.x + 2048 * clut.y) as isize;

        if self.clut_cache_tag != clut_address {
            for i in 0..16 {
                let address = (clut_address as usize) + 2 * i;
                self.clut_cache[i] = LittleEndian::read_u16(&self.vram[address..]);
            }

            self.clut_cache_tag = clut_address;
        }

        let texture = self.clut_cache[clut_entry];
        (Colour::from_u16(texture), texture == 0)
    }

    fn read_clut_8bit(&mut self, uv: Vector2i, clut: Vector2i) -> (Colour, bool) {
        let address_x = 2 * self.texpage.x_base + (uv.x & 0xff) as u32;
        let address_y = self.texpage.y_base + (uv.y & 0xff) as u32;
        let texture_address = (address_x + 2048 * address_y) as usize;

        let block = (((uv.y >> 6) << 3) + (uv.x >> 5)) as isize;
        let entry = (((uv.y & 0x3f) << 2) + ((uv.x & 0x1f) >> 3)) as usize;

        let index = (uv.x & 0x7) as usize;

        let centry = &mut self.texture_cache[entry];

        if centry.tag != block {
            for i in 0..8 {
                centry.data[i] = self.vram[(texture_address & !0x7) + i];
            }

            centry.tag = block;
        }

        let clut_entry = centry.data[index] as usize;

        let clut_address = (2 * clut.x + 2048 * clut.y) as isize;

        if self.clut_cache_tag != clut_address {
            for i in 0..256 {
                let address = (clut_address as usize) + 2 * i;
                self.clut_cache[i] = LittleEndian::read_u16(&self.vram[address..]);
            }

            self.clut_cache_tag = clut_address;
        }

        let texture = self.clut_cache[clut_entry];
        (Colour::from_u16(texture), texture == 0)
    }

    fn read_texture(&mut self, uv: Vector2i) -> (Colour, bool) {
        let address_x = self.texpage.x_base + (uv.x & 0xff) as u32;
        let address_y = self.texpage.y_base + (uv.y & 0xff) as u32;
        let texture_address = 2 * (address_x + 1024 * address_y) as usize;

        let block = (((uv.y >> 5) << 3) + (uv.x >> 5)) as isize;
        let entry = (((uv.y & 0x1f) << 3) + ((uv.x & 0x1f) >> 2)) as usize;

        let index = ((uv.x * 2) & 0x7) as usize;

        let centry = &mut self.texture_cache[entry];

        if centry.tag != block {
            for i in 0..8 {
                centry.data[i] = self.vram[(texture_address & !0x7) + i];
            }

            centry.tag = block;
        }

        let texture = LittleEndian::read_u16(&centry.data[index..]);
        (Colour::from_u16(texture), texture == 0)
    }
}