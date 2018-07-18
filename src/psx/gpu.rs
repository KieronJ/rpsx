use std::cell::RefCell;
use std::cmp;
use std::mem;
use std::rc::Rc;

use byteorder::{LittleEndian, ByteOrder};

use queue::Queue;

use super::controller::Controller;
use super::display::Display;
use super::rasteriser::{Colour, Vector2i, Vector3f};
use super::timer::Timer;

pub const DITHER_TABLE: [isize; 16] = [-4,  0, -3,  1,
                                        2, -2,  3, -1,
                                       -3,  1, -4,  0,
                                        3, -1,  2, -2];

pub const CMD_SIZE: [usize; 256] = [
    1, 1, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1,  1,  1,  1,  1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,  1,  1,  1,  1,
    4, 4, 4, 4, 7, 7, 7, 7, 5, 5, 5, 5,  9,  9,  9,  9,
    6, 6, 6, 6, 9, 9, 9, 9, 8, 8, 8, 8, 12, 12, 12, 12,
    3, 1, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1,  1,  1,  1,  1,
    4, 1, 4, 1, 1, 1, 1, 1, 1, 1, 1, 1,  1,  1,  1,  1,
    3, 1, 3, 1, 4, 4, 4, 4, 2, 1, 2, 1,  3,  3,  3,  3,
    2, 1, 2, 1, 3, 3, 3, 3, 2, 1, 2, 1,  3,  3,  3,  3,
    4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,  4,  4,  4,  4,
    4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,  4,  4,  4,  4,
    3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,  3,  3,  3,  3,
    3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,  3,  3,  3,  3,
    3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,  3,  3,  3,  3,
    3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,  3,  3,  3,  3,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,  1,  1,  1,  1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,  1,  1,  1,  1,
];

struct GpuTransfer {
    x: u32,
    y: u32,
    w: u32,
    h: u32,

    rx: u32,
    ry: u32,

    active: bool,
}

impl GpuTransfer {
    pub fn new() -> GpuTransfer {
        GpuTransfer {
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

#[derive(Clone, Copy)]
enum DmaDirection {
    Off,
    Fifo,
    CpuToGp0,
    GpureadToCpu,
}

#[derive(Clone, Copy)]
enum TexturePageColours {
    TP4Bit,
    TP8Bit,
    TP15Bit,
    Reserved,
}

#[derive(Clone, Copy)]
enum SemiTransparency {
    Half,
    Add,
    Subtract,
    AddQuarter,
}

#[derive(Clone, Copy)]
struct GpuTexpage {
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

impl GpuTexpage {
    pub fn new() -> GpuTexpage {
        GpuTexpage {
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

    pub fn from_u32(value: u32) -> GpuTexpage {
        let texpage = value >> 16;

        GpuTexpage {
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

pub struct Gpu {
    display: Display,
    timer: Rc<RefCell<Timer>>,

    vram: Box<[u8]>,

    scanline: usize,
    video_cycle: usize,

    dotclock_cycle: usize,

    gpuread: u32,

    command_buffer: Queue<u32>,
    command_words_remaining: usize,

    cpu_to_gpu_transfer: GpuTransfer,
    gpu_to_cpu_transfer: GpuTransfer,

    interlace_line: bool,
    dma_direction: DmaDirection,

    dma_ready: bool,
    vram_ready: bool,
    cmd_ready: bool,

    //TODO: DMA/Data Request

    irq: bool,

    display_disable: bool,
    vertical_interlace: bool,
    colour_depth: bool,
    video_mode: bool,

    vres: u32,
    hres: u32,

    reverse: bool,

    skip_masked_pixels: bool,
    set_mask_bit: bool,

    texpage: GpuTexpage,

    rectangle: bool,

    shaded: bool,
    semi_tranparent: bool,
    blending: bool,
    textured: bool,

    drawing_begin: u32,
    drawing_x_begin: u32,
    drawing_y_begin: u32,

    drawing_end: u32,
    drawing_x_end: u32,
    drawing_y_end: u32,

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
}

impl Gpu {
    pub fn new(controller: Rc<RefCell<Controller>>, timer: Rc<RefCell<Timer>>) -> Gpu {
        Gpu {
            display: Display::new(1280, 960, "rpsx", controller),
            timer: timer,

            vram: vec![0; 0x100000].into_boxed_slice(),

            scanline: 0,
            video_cycle: 0,

            dotclock_cycle: 0,

            gpuread: 0,

            command_buffer: Queue::<u32>::new(16),
            command_words_remaining: 0,

            cpu_to_gpu_transfer: GpuTransfer::new(),
            gpu_to_cpu_transfer: GpuTransfer::new(),

            interlace_line: false,
            dma_direction: DmaDirection::Off,

            dma_ready: true,
            vram_ready: true,
            cmd_ready: true,

            //TODO: DMA/Data Request

            irq: false,

            display_disable: false,
            vertical_interlace: false,
            colour_depth: false,
            video_mode: false,

            vres: 240,
            hres: 320,

            reverse: false,

            skip_masked_pixels: false,
            set_mask_bit: false,

            texpage: GpuTexpage::new(),

            rectangle: false,

            shaded: false,
            semi_tranparent: false,
            blending: false,
            textured: false,

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
        }
    }

    pub fn tick(&mut self, clocks: usize) -> bool {
        let cycles = self.horizontal_length();
        let scanlines = self.vertical_length();
        let dotclock = self.get_dotclock() as usize;

        let mut irq = false;

        self.video_cycle += clocks;
        self.dotclock_cycle += clocks;

        {
            let mut timer = self.timer.borrow_mut();
            timer.tick_dotclock(self.dotclock_cycle / dotclock);
        }

        self.dotclock_cycle %= dotclock;

        let old_hblank = self.in_hblank();
        let old_vblank = self.in_vblank();

        if self.video_cycle >= cycles {
            self.video_cycle -= cycles;

            self.scanline += 1;
            if self.scanline >= scanlines {
                self.scanline = 0;

                self.render_frame();
                irq = true;
            }
        }

        let mut timer = self.timer.borrow_mut();

        if self.in_hblank() {
            if !old_hblank {
                timer.set_hblank(true);
                timer.tick_hblank();
            }
        } else {
            if old_hblank {
                timer.set_hblank(false);
            }
        }

        if self.in_vblank() {
            if !old_vblank {
                timer.set_vblank(true);
            }
        } else {
            if old_vblank {
                timer.set_vblank(false);
            }
        }

        irq
    }

    pub fn irq(&self) -> bool {
        self.irq
    }

    fn horizontal_length(&self) -> usize {
        match self.video_mode {
            true => 3406,
            false => 3413,
        }
    }

    fn vertical_length(&self) -> usize {
        match self.video_mode {
            true => 314,
            false => 263,
        }
    }

    pub fn in_hblank(&self) -> bool {
        self.video_cycle < self.horizontal_display_start as usize ||
        self.video_cycle > self.horizontal_display_end as usize
    }

    pub fn in_vblank(&self) -> bool {
        self.scanline < self.vertical_display_start as usize ||
        self.scanline > self.vertical_display_end as usize
    }

    pub fn get_dotclock(&self) -> u32 {
        match self.hres {
            320 => 8,
            640 => 4,
            256 => 10,
            512 => 5,
            368 => 6,
            _ => unreachable!(),
        }
    }

    fn dither_get_offset(x: usize, y: usize) -> isize {
        DITHER_TABLE[(x & 0x3) + (y & 0x3) * 4]
    }

    fn render_frame(&mut self) {
        let start_x = self.display_area_x as usize;
        let start_y = self.display_area_y as usize;

        let start_address = 2 * (start_x + start_y * 1024);

        self.display.draw(&self.vram[start_address..]);
        self.display.handle_events();
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

        value |= (self.interlace_line as u32) << 31;
        value |= (self.dma_direction as u32) << 29;
        value |= (self.dma_ready as u32) << 28;
        value |= (self.vram_ready as u32) << 27;
        value |= (self.cmd_ready as u32) << 26;
        // TODO: DMA / Data Request
        value |= (self.irq as u32) << 24;
        value |= (self.display_disable as u32) << 23;
        value |= (self.vertical_interlace as u32) << 22;
        value |= (self.colour_depth as u32) << 21;
        value |= (self.video_mode as u32) << 20;
        value |= match self.vres {
            480 => (1 << 19),
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
        };
        value |= (self.texpage.texture_disable as u32) << 15;
        value |= (self.reverse as u32) << 14;
        value |= (self.vertical_interlace as u32) << 13;
        value |= (self.skip_masked_pixels as u32) << 12;
        value |= (self.set_mask_bit as u32) << 11;
        value |= (self.texpage.display_area_enable as u32) << 10;
        value |= (self.texpage.dithering_enable as u32) << 9;
        value |= (self.texpage.colour_depth as u32) << 7;
        value |= (self.texpage.semi_transparency as u32) << 5;
        value |= self.texpage.y_base / 16;
        value |= self.texpage.x_base / 64;

        self.interlace_line = !self.interlace_line;

        value
    }

    pub fn gp0_write(&mut self, word: u32) {
        if self.cpu_to_gpu_transfer.active {
            self.vram_write_transfer(word as u16);
            self.vram_write_transfer((word >> 16) as u16);

            return;
        }

        self.push_gp0_command(word);
    }

    fn vram_address(x: u32, y: u32) -> usize {
        2 * (x + y * 1024) as usize
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

        let destination_address = Gpu::vram_address(x & 0x3ff, y & 0x1ff);
        LittleEndian::read_u16(&self.vram[destination_address..])
    }

    fn vram_write_transfer(&mut self, data: u16) {
        let x = self.cpu_to_gpu_transfer.x + self.cpu_to_gpu_transfer.rx;
        let y = self.cpu_to_gpu_transfer.y + self.cpu_to_gpu_transfer.ry;

        let destination_address = Gpu::vram_address(x & 0x3ff, y & 0x1ff);
        LittleEndian::write_u16(&mut self.vram[destination_address..], data);

        self.cpu_to_gpu_transfer.rx += 1;

        if self.cpu_to_gpu_transfer.rx == self.cpu_to_gpu_transfer.w {
            self.cpu_to_gpu_transfer.rx = 0;
            
            self.cpu_to_gpu_transfer.ry += 1;

            if self.cpu_to_gpu_transfer.ry == self.cpu_to_gpu_transfer.h {
                self.cpu_to_gpu_transfer.ry = 0;
                self.cpu_to_gpu_transfer.active = false;
            }
        }
    }

    fn push_gp0_command(&mut self, command_word: u32) {
        if !self.command_buffer.full() {
            self.command_buffer.push(command_word);
        }

        if self.command_buffer.full() {
            self.cmd_ready = false;
        }

        if self.command_words_remaining == 0 {
            let command = (command_word >> 24) as usize;
            self.command_words_remaining = CMD_SIZE[command];
        }
        
        if self.command_words_remaining == 1 {
            self.execute_gp0_command();
        }

        self.command_words_remaining -= 1;
    }

    fn execute_gp0_command(&mut self) {
        let command_word = self.command_buffer.pop();
        let command = command_word >> 24;

        match command {
            0x00 => {}, // NOP
            0x01 => {
                // TODO: Clear Cache
            },
            0x02 => {
                let destination = self.command_buffer.pop();
                let size = self.command_buffer.pop();

                let colour = Colour::from_u32(command_word);
                let pixel = colour.to_u16();

                let x_start = destination & 0x3f0;
                let y_start = (destination >> 16) & 0x1ff;
                
                let w = ((size & 0x3f0) + 0xf) & !0xf;
                let h = (size >> 16) & 0x1ff;

                let x_end = x_start + w;
                let y_end = y_start + h;

                for y in y_start..y_end {
                    for x in x_start..x_end {
                        let destination_address = Gpu::vram_address(x & 0x3ff, y & 0x1ff);
                        LittleEndian::write_u16(&mut self.vram[destination_address..], pixel);
                    }
                }
            },
            0x03...0x1e => {}, // NOP
            0x1f => self.irq = true,
            0x20...0x3f => self.draw_polygon(command_word),
            0x40...0x5f => (), //self.draw_line(command_word),
            0x60...0x7f => self.draw_rectangle(command_word),
            0x80...0x9f => {
                let src = self.command_buffer.pop();
                let dest = self.command_buffer.pop();
                let size = self.command_buffer.pop();

                let src_x = src & 0x3ff;
                let src_y = (src >> 16) & 0x1ff;
                let dest_x = dest & 0x3ff;
                let dest_y = (dest >> 16) & 0x1ff;
                let w = size & 0x3ff;
                let h = (size >> 16) & 0x1ff;

                for y in src_y..src_y + h {
                    for x in src_x..src_x + w {
                        let src_address = Gpu::vram_address((src_x + x) & 0x3ff, (src_y + y) & 0x1ff);
                        let dest_address = Gpu::vram_address((dest_x + x) & 0x3ff, (dest_y + y) & 0x1ff);

                        let data = LittleEndian::read_u16(&self.vram[src_address..]);
                        LittleEndian::write_u16(&mut self.vram[dest_address..], data);
                    }
                }
            },
            0xa0...0xbf => {
                let destination = self.command_buffer.pop();
                let size = self.command_buffer.pop();

                let x = destination & 0x3ff;
                let y = (destination >> 16) & 0x1ff;
                let w = size & 0x3ff;
                let h = (size >> 16) & 0x1ff;

                self.cpu_to_gpu_transfer.x = x;
                self.cpu_to_gpu_transfer.y = y;
                self.cpu_to_gpu_transfer.w = w;
                self.cpu_to_gpu_transfer.h = h;

                self.cpu_to_gpu_transfer.rx = 0;
                self.cpu_to_gpu_transfer.ry = 0;

                self.cpu_to_gpu_transfer.active = true;
            },
            0xc0...0xdf => {
                let destination = self.command_buffer.pop();
                let size = self.command_buffer.pop();

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
            },
            0xe0 => {}, // NOP
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
            },
            0xe2 => {
                self.texture_window = command_word & 0xf_ffff;
                self.texture_window_offset_y = ((command_word & 0xf_8000) >> 15) * 8;
                self.texture_window_offset_x = ((command_word & 0x7c00) >> 10) * 8;
                self.texture_window_mask_y = ((command_word & 0x3e0) >> 5) * 8;
                self.texture_window_mask_x = (command_word & 0x1f) * 8;
            },
            0xe3 => {
                self.drawing_begin = command_word & 0x7_ffff;
                self.drawing_y_begin = (command_word & 0xf_fc00) >> 10;
                self.drawing_x_begin = command_word & 0x3ff;
            },
            0xe4 => {
                self.drawing_end = command_word & 0x7_ffff;
                self.drawing_y_end = (command_word & 0xf_fc00) >> 10;
                self.drawing_x_end = command_word & 0x3ff;
            },
            0xe5 => {
                self.drawing_offset = command_word & 0x3f_ffff;

                let mut dyo = (command_word >> 11) & 0x7ff;
                let mut dxo = command_word & 0x7ff;

                if (dyo & 0x800) != 0 {
                    dyo |= 0xffff_f800; 
                }

                if (dxo & 0x800) != 0 {
                    dxo |= 0xffff_f800; 
                }

                self.drawing_y_offset = dyo as i32;
                self.drawing_x_offset = dxo as i32;
            },
            0xe6 => {
                self.skip_masked_pixels = (command_word & 0x2) != 0;
                self.set_mask_bit = (command_word & 0x1) != 0;
            },
            0xe7...0xff => {}, // NOP
            _ => panic!("[GPU] [ERROR] Unknown command GP0({:02x})", command),
        }
    }

    fn shaded(command: usize) -> bool {
        (command & 0x10) != 0
    }

    fn vertices(command: usize) -> (usize, usize) {
        match (command & 0x08) != 0 {
            false => (1, 3),
            true => (2, 4),
        }
    }

    fn rect_size(command: usize) -> i32 {
        match (command & 0x18) >> 3 {
            0x0 => 0,
            0x1 => 1,
            0x2 => 8,
            0x3 => 16,
            _ => unreachable!(),
        }
    }

    fn textured(command: usize) -> bool {
        (command & 0x04) != 0
    }

    fn semi_tranparent(command: usize) -> bool {
        (command & 0x02) != 0
    }

    fn blending(command: usize) -> bool {
        (command & 0x01) == 0
    }

    fn get_colour(word: u32) -> Colour {
        Colour::from_u32(word)
    }

    fn get_coord(&self, word: u32) -> Vector2i {
        let xo = self.drawing_x_offset;
        let yo = self.drawing_y_offset;

        let mut x = (word & 0xffff) as u16;
        let mut y = (word >> 16) as u16;

            if (y & 0x400) != 0 {
                y |= 0xfc00; 
            }

            if (x & 0x400) != 0 {
                x |= 0x_fc00; 
            }

        Vector2i::new(xo + (x as i16 as i32), yo + (y as i16 as i32))
    }

    fn get_texcoord(word: u32) -> Vector2i {
        let x = (word & 0xff) as i32;
        let y = ((word >> 8) & 0xff) as i32;

        Vector2i::new(x, y)
    }

    fn get_clut(word: u32) -> Vector2i {
        let clut = word >> 16;

        let x = ((clut & 0x3f) << 4) as i32;
        let y = ((clut >> 6) & 0x1ff) as i32;

        Vector2i::new(x, y)
    }

    fn draw_polygon(&mut self, command_word: u32) {
        let command = (command_word >> 24) as usize;

        self.rectangle = false;

        self.shaded = Gpu::shaded(command);
        self.semi_tranparent = Gpu::semi_tranparent(command);
        self.blending = Gpu::blending(command);
        self.textured = Gpu::textured(command);

        let (polygons, vertices) = Gpu::vertices(command);

        let mut coords = [Vector2i::new(0, 0); 4];
        let mut texcoords = [Vector2i::new(0, 0); 4];
        let mut colours = [Gpu::get_colour(command_word); 4];

        let mut clut = Vector2i::new(0, 0);
        let mut texpage = GpuTexpage::new();

        for i in 0..vertices {
            if self.shaded && (i != 0) {
                colours[i] = Gpu::get_colour(self.command_buffer.pop());
            }

            let coord = self.command_buffer.pop();
            coords[i] = self.get_coord(coord);

            if self.textured {
                let texcoord = self.command_buffer.pop();
                texcoords[i] = Gpu::get_texcoord(texcoord);

                if i == 0 {
                    clut = Gpu::get_clut(texcoord);
                } else if i == 1 {
                    texpage = GpuTexpage::from_u32(texcoord);
                }
            }
        }

        self.rasterise_triangle(&coords[0..3], &texcoords[0..3], &colours[0..3], clut, texpage);

        if polygons == 2 {
            self.rasterise_triangle(&coords[1..4], &texcoords[1..4], &colours[1..4], clut, texpage);
        }
    }

    fn draw_rectangle(&mut self, command_word: u32) {
        let command = (command_word >> 24) as usize;

        let size = Gpu::rect_size(command);
        
        self.rectangle = true;

        self.shaded = false;
        self.semi_tranparent = Gpu::semi_tranparent(command);
        self.blending = Gpu::blending(command);
        self.textured = Gpu::textured(command);

        let mut coords = [Vector2i::new(0, 0); 4];

        let colours = [Colour::from_u32(command_word); 4];

        let mut texcoords = [Vector2i::new(0, 0); 4];
        let mut clut = Vector2i::new(0, 0);
        let texpage = self.texpage;

        let coord = self.command_buffer.pop();
        coords[0] = self.get_coord(coord);

        if self.textured {
            let t = self.command_buffer.pop();
            texcoords[0] = Gpu::get_texcoord(t);

            clut = Gpu::get_clut(t);
        }

        let (w, h) = match size {
            0 => {
                let rect_size = self.command_buffer.pop();
                ((rect_size & 0xffff) as i16 as i32, (rect_size >> 16) as i16 as i32)
            },
            _ => (size, size),
        };

        coords[1] = Vector2i::new(coords[0].x + w,  coords[0].y);
        coords[2] = Vector2i::new(coords[0].x,      coords[0].y + h);
        coords[3] = Vector2i::new(coords[0].x + w,  coords[0].y + h);

        texcoords[1] = Vector2i::new(texcoords[0].x + w,    texcoords[0].y);
        texcoords[2] = Vector2i::new(texcoords[0].x,        texcoords[0].y + h);
        texcoords[3] = Vector2i::new(texcoords[0].x + w,    texcoords[0].y + h);

        self.rasterise_triangle(&coords[0..3], &texcoords[0..3], &colours[0..3], clut, texpage);
        self.rasterise_triangle(&coords[1..4], &texcoords[1..4], &colours[1..4], clut, texpage);
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

                self.update_video_mode();

                self.skip_masked_pixels = false;
                self.set_mask_bit = false;
            },
            0x01 => {
                self.command_buffer.clear();
            },
            0x02 => {
                self.irq = false;
            },
            0x03 => {
                self.display_disable = (command_word & 0x1) != 0;
            },
            0x04 => {
                self.dma_direction = match command_word & 0x3 {
                    0 => DmaDirection::Off,
                    1 => DmaDirection::Fifo,
                    2 => DmaDirection::CpuToGp0,
                    3 => DmaDirection::GpureadToCpu,
                    _ => unreachable!(),
                };
            },
            0x05 => {
                self.display_area_y = (command_word & 0x7_fc00) >> 10;
                self.display_area_x = command_word & 0x3ff;
            },
            0x06 => {
                self.horizontal_display_end = (command_word & 0xff_f000) >> 12;
                self.horizontal_display_start = command_word & 0xfff;

                self.update_video_mode();
            },
            0x07 => {
                self.vertical_display_end = (command_word & 0xf_fc00) >> 10;
                self.vertical_display_start = command_word & 0x3ff;

                self.update_video_mode();
            },
            0x08 => {
                self.reverse = (command_word & 0x80) != 0;
                self.vertical_interlace = (command_word & 0x20) != 0;
                self.colour_depth = (command_word & 0x10) != 0;

                if self.colour_depth {
                    panic!("[GPU] [ERROR] Unsupported display colour depth: 24bit.");
                }

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

                self.update_video_mode();
            },
            0x09 => (), //New Texture Disable
            0x10...0x1f => {
                match command_word & 0x07 {
                    0x02 => self.gpuread = self.texture_window,
                    0x03 => self.gpuread = self.drawing_begin,
                    0x04 => self.gpuread = self.drawing_end,
                    0x05 => self.gpuread = self.drawing_offset,
                    _ => (),
                };
            },
            0x20 => (), //Arcade Texture Disable
            _ => panic!("[GPU] [ERROR] Unknown command GP1({:02x})", command),
        }
    }

    fn rasterise_pixel(&mut self, p: Vector2i, c: Colour, texpage: GpuTexpage) {
        let x = p.x as usize;
        let y = p.y as usize;
        
        let mut colour = c;

        if self.semi_tranparent && colour.a {
            colour = self.blend(colour, x, y, texpage)
        }

        if !self.rectangle && (self.shaded || self.textured) {
            colour = self.dither(colour, x, y);
        }

        let pixel = colour.to_u16();

        self.vram[2 * (x + y * 1024)] = pixel as u8;
        self.vram[2 * (x + y * 1024) + 1] = (pixel >> 8) as u8;
    }

    fn dither(&self, mut colour: Colour, x: usize, y: usize) -> Colour {
        if !self.texpage.dithering_enable {
            return colour;
        }

        let dither_offset = (Gpu::dither_get_offset(x, y) as f32) / 255.0;

        colour.r = Gpu::clamp(colour.r + dither_offset, 0.0, 1.0);
        colour.g = Gpu::clamp(colour.g + dither_offset, 0.0, 1.0);
        colour.b = Gpu::clamp(colour.b + dither_offset, 0.0, 1.0);

        colour
    }

    fn clamp(value: f32, min: f32, max: f32) -> f32 {
        if value < min {
            return min;
        }

        if value > max {
            return max;
        }

        value
    }

    fn blend(&self, front: Colour, x: usize, y: usize, texpage: GpuTexpage) -> Colour {
        let mut back = Colour::from_u16(LittleEndian::read_u16(&self.vram[2 * (x + y * 1024)..]));

        match texpage.semi_transparency {
            SemiTransparency::Half => {
                back.r = (back.r * front.r) / 2.0;
                back.g = (back.g * front.g) / 2.0;
                back.b = (back.b * front.b) / 2.0;
            },
            SemiTransparency::Add => {
                back.r += front.r;
                back.g += front.g;
                back.b += front.b;
            },
            SemiTransparency::Subtract => {
                back.r -= front.r;
                back.g -= front.g;
                back.b -= front.b;
            },
            SemiTransparency::AddQuarter => {
                back.r += front.r / 4.0;
                back.g += front.g / 4.0;
                back.b += front.b / 4.0;
            },
        };

        back.r = Gpu::clamp(back.r, 0.0, 1.0);
        back.g = Gpu::clamp(back.g, 0.0, 1.0);
        back.b = Gpu::clamp(back.b, 0.0, 1.0);

        back
    }

    fn bounding_box(&self, a: Vector2i, b: Vector2i, c: Vector2i) -> (Vector2i, Vector2i) {
        let mut min = Vector2i::new(0, 0);
        let mut max = Vector2i::new(0, 0);

        let min_draw_x = cmp::max(0, self.drawing_x_begin as i32);
        let min_draw_y = cmp::max(0, self.drawing_y_begin as i32);

        let max_draw_x = cmp::min(1025, self.drawing_x_end as i32);
        let max_draw_y = cmp::min(513, self.drawing_y_end as i32);

        let min_x = cmp::min(a.x, cmp::min(b.x, c.x));
        let min_y = cmp::min(a.y, cmp::min(b.y, c.y));

        let max_x = cmp::max(a.x, cmp::max(b.x, c.x));
        let max_y = cmp::max(a.y, cmp::max(b.y, c.y));

        min.x = cmp::max(min_x, min_draw_x);
        min.y = cmp::max(min_y, min_draw_y);

        max.x = cmp::min(max_x, max_draw_x);
        max.y = cmp::min(max_y, max_draw_y);

        (min, max)
    }

    fn rasterise_triangle(&mut self, p: &[Vector2i], t: &[Vector2i], c: &[Colour], clut: Vector2i, texpage: GpuTexpage) {
        let p0 = p[0];
        let mut p1 = p[1];
        let mut p2 = p[2];

        let mut tex = [Vector2i::new(0, 0); 3];
        tex[0] = t[0];
        tex[1] = t[1];
        tex[2] = t[2];

        let mut col = [Colour::new(0.0, 0.0, 0.0, false); 3];
        col[0] = c[0];
        col[1] = c[1];
        col[2] = c[2];

        let mut area = Vector2i::orient2d(p0, p1, p2) as f32;

        if area < 0.0 {
            mem::swap(&mut p1, &mut p2);

            tex.swap(1, 2);
            col.swap(1, 2);

            area *= -1.0;

        } else if area == 0.0 {
            return;
        }

        let (min, max) = self.bounding_box(p0, p1, p2);

        let a01 = p0.y - p1.y; let b01 = p1.x - p0.x;
        let a12 = p1.y - p2.y; let b12 = p2.x - p1.x;
        let a20 = p2.y - p0.y; let b20 = p0.x - p2.x;

        let mut w0_row = Vector2i::orient2d(p1, p2, min);
        let mut w1_row = Vector2i::orient2d(p2, p0, min);
        let mut w2_row = Vector2i::orient2d(p0, p1, min);

        for y in min.y..max.y {
            let mut w0 = w0_row;
            let mut w1 = w1_row;
            let mut w2 = w2_row;

            for x in min.x..max.x {
                if (w0 | w1 | w2) >= 0 {
                    let v = Vector2i::new(x, y);

                    let mut colour = match self.shaded {
                        false => c[0],
                        true => {
                            let w = Vector3f::new(w0 as f32 / area, w1 as f32 / area, w2 as f32 / area);
                            Gpu::get_shade(&col, w)
                        },
                    };

                    if self.textured {
                        let w = Vector3f::new(w0 as f32 / area, w1 as f32 / area, w2 as f32 / area);
                        let (mut texture, skip) = self.get_texture(&tex, w, clut, texpage);

                        if skip {
                            w0 -= a12;
                            w1 -= a20;
                            w2 -= a01;
                            continue;
                        }

                        if self.blending {
                            texture.r = Gpu::clamp(texture.r * colour.r * 2.0, 0.0, 1.0);
                            texture.g = Gpu::clamp(texture.g * colour.g * 2.0, 0.0, 1.0);
                            texture.b = Gpu::clamp(texture.b * colour.b * 2.0, 0.0, 1.0);
                        }

                        colour = texture;
                    };

                    self.rasterise_pixel(v, colour, texpage);
                }

                w0 -= a12;
                w1 -= a20;
                w2 -= a01;
            }

            w0_row -= b12;
            w1_row -= b20;
            w2_row -= b01;
        }
    }

    fn get_shade(c: &[Colour], w: Vector3f) -> Colour {
        Colour::interpolate_colour(c, w)
    }

    fn get_texture(&self, t: &[Vector2i], w: Vector3f, clut: Vector2i, texpage: GpuTexpage) -> (Colour, bool) {
        use self::TexturePageColours::*;

        let v = Vector2i::interpolate_texcoord(t, w);

        let pixel = match texpage.colour_depth {
            TP4Bit => self.read_clut_4bit(v, texpage, clut),
            TP8Bit => self.read_clut_8bit(v, texpage, clut),
            TP15Bit | Reserved => self.read_texpage(v, texpage),
        };
        
        (Colour::from_u16(pixel), pixel == 0)
    }

    fn read_texpage(&self, v: Vector2i, texpage: GpuTexpage) -> u16 {
        let x = texpage.x_base + ((v.x as u32) & 0xff);
        let y = texpage.y_base + ((v.y as u32) & 0xff);

        LittleEndian::read_u16(&self.vram[Gpu::vram_address(x & 0x3ff, y & 0x1ff)..])
    }

    fn read_clut_4bit(&self, v: Vector2i, texpage: GpuTexpage, clut: Vector2i) -> u16 {
        let texture_address = (2 * texpage.x_base + ((v.x as u32)) / 2 + (texpage.y_base + ((v.y as u32))) * 2048) as usize;
        let mut clut_index = self.vram[texture_address] as usize;

        if v.x & 0x1 != 0 {
            clut_index >>= 4;
        } else {
            clut_index &= 0x0f;
        }

        let clut_address = 2 * (clut.x + clut.y * 1024) as usize;
        let indexed_clut = clut_address + clut_index * 2;

        LittleEndian::read_u16(&self.vram[indexed_clut..])
    }

    fn read_clut_8bit(&self, v: Vector2i, texpage: GpuTexpage, clut: Vector2i) -> u16 {
        let texture_address = (2 * texpage.x_base + ((v.x as u32)) + (texpage.y_base + ((v.y as u32))) * 2048) as usize;
        let clut_index = self.vram[texture_address] as usize;

        let clut_address = 2 * (clut.x + clut.y * 1024) as usize;
        let indexed_clut = clut_address + clut_index * 2;

        LittleEndian::read_u16(&self.vram[indexed_clut..])
    }

    fn update_video_mode(&mut self) {
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

        let x = ((xdiff / dotclock) + 2) & 0xffc;
        let mut y = yend - ystart;

        if self.vertical_interlace {
            y <<= 1;
        }

        self.display.update_video_mode(x, y);
    }
}