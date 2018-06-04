use byteorder::{LittleEndian, ByteOrder};

use super::display::Display;
use super::rasteriser::{Colour, Quad, Triangle, Vector2f, Vertex};

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

struct GpuQueue {
    commands: [u32; 16],
    length: usize,
}

impl GpuQueue {
    pub fn new() -> GpuQueue {
        GpuQueue {
            commands: [0; 16],
            length: 0,
        }
    }

    pub fn push(&mut self, value: u32) {
        self.commands[self.length] = value;

        if !self.full() {
            self.length += 1;
        }
    }

    pub fn pop(&mut self) -> u32 {
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

#[derive(Clone, Copy)]
enum DmaDirection {
    Off,
    Fifo,
    CpuToGp0,
    GpureadToCpu,
}

#[derive(Clone, Copy)]
enum HorizontalResolution {
    X256,
    X320,
    X512,
    X640,
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
    ST00,
    ST01,
    ST10,
    ST11,
}

#[derive(Clone, Copy)]
enum TexturePageXBase {
    TPX0000,
    TPX0001,
    TPX0010,
    TPX0011,
    TPX0100,
    TPX0101,
    TPX0110,
    TPX0111,
    TPX1000,
    TPX1001,
    TPX1010,
    TPX1011,
    TPX1100,
    TPX1101,
    TPX1110,
    TPX1111,
}

struct GpuStatus {
    line_odd: bool,
    dma_direction: DmaDirection,
    dma_ready: bool,
    vram_ready: bool,
    cmd_ready: bool,
    // TODO: DMA / Data Request
    irq: bool,
    display_disable: bool,
    vertical_interlace: bool,
    display_colour_depth: bool,
    video_mode: bool,
    vertical_resolution: bool,
    horizontal_resolution_1: HorizontalResolution,
    horizontal_resolution_2: bool,
    texture_disable: bool,
    reverse: bool,
    // TODO: Interlace field
    draw_pixels: bool,
    set_mask_bit: bool,
    display_area_enable: bool,
    dither: bool,
    texture_page_colours: TexturePageColours,
    semi_transparency: SemiTransparency,
    texture_page_y_base: bool,
    texture_page_x_base: TexturePageXBase,
}

impl GpuStatus {
    pub fn new() -> GpuStatus {
        GpuStatus {
            line_odd: false,
            dma_direction: DmaDirection::Off,
            dma_ready: true,
            vram_ready: true,
            cmd_ready: true,
            // TODO: DMA / Data Request
            irq: false,
            display_disable: false,
            vertical_interlace: false,
            display_colour_depth: false,
            video_mode: false,
            vertical_resolution: false,
            horizontal_resolution_1: HorizontalResolution::X256,
            horizontal_resolution_2: false,
            texture_disable: false,
            reverse: false,
            // TODO: Interlace field
            draw_pixels: false,
            set_mask_bit: false,
            display_area_enable: false,
            dither: false,
            texture_page_colours: TexturePageColours::TP4Bit,
            semi_transparency: SemiTransparency::ST00,
            texture_page_y_base: false,
            texture_page_x_base: TexturePageXBase::TPX0000,
        }
    }

    pub fn read(&mut self) -> u32 {
        let mut value = 0;

        value |= (self.line_odd as u32) << 31;
        value |= (self.dma_direction as u32) << 29;
        value |= (self.dma_ready as u32) << 28;
        value |= (self.vram_ready as u32) << 27;
        value |= (self.cmd_ready as u32) << 26;
        // TODO: DMA / Data Request
        value |= (self.irq as u32) << 24;
        value |= (self.display_disable as u32) << 23;
        value |= (self.vertical_interlace as u32) << 22;
        value |= (self.display_colour_depth as u32) << 21;
        value |= (self.video_mode as u32) << 20;
        value |= (self.vertical_resolution as u32) << 19;
        value |= (self.horizontal_resolution_1 as u32) << 17;
        value |= (self.horizontal_resolution_2 as u32) << 16;
        value |= (self.texture_disable as u32) << 15;
        value |= (self.reverse as u32) << 14;
        // TODO: Interlace field
        value |= (self.draw_pixels as u32) << 12;
        value |= (self.set_mask_bit as u32) << 11;
        value |= (self.display_area_enable as u32) << 10;
        value |= (self.dither as u32) << 9;
        value |= (self.texture_page_colours as u32) << 7;
        value |= (self.semi_transparency as u32) << 5;
        value |= (self.texture_page_y_base as u32) << 4;
        value |= self.texture_page_x_base as u32;

        self.line_odd = !self.line_odd;

        value
    }
}

pub struct Gpu {
    display: Display,

    vram: Box<[u8]>,

    command_buffer: GpuQueue,
    command_words_remaining: usize,

    cpu_to_gpu_transfer: GpuTransfer,
    gpu_to_cpu_transfer: GpuTransfer,

    status: GpuStatus,

    drawing_x_begin: u32,
    drawing_x_end: u32,
    drawing_x_offset: u32,

    drawing_y_begin: u32,
    drawing_y_end: u32,
    drawing_y_offset: u32,

    texture_window_mask_x: u32,
    texture_window_offset_x: u32,

    texture_window_mask_y: u32,
    texture_window_offset_y: u32,

    display_area_x: u32,
    display_area_y: u32,

    textured_rectangle_x_flip: bool,
    textured_rectangle_y_flip: bool,
}

impl Gpu {
    pub fn new() -> Gpu {
        Gpu {
            display: Display::new(256, 240, "rpsx"),

            vram: vec![0; 0x100000].into_boxed_slice(),

            command_buffer: GpuQueue::new(),
            command_words_remaining: 0,

            cpu_to_gpu_transfer: GpuTransfer::new(),
            gpu_to_cpu_transfer: GpuTransfer::new(),

            status: GpuStatus::new(),

            drawing_x_begin: 0,
            drawing_x_end: 0,
            drawing_x_offset: 0,

            drawing_y_begin: 0,
            drawing_y_end: 0,
            drawing_y_offset: 0,

            texture_window_mask_x: 0,
            texture_window_offset_x: 0,

            texture_window_mask_y: 0,
            texture_window_offset_y: 0,

            display_area_x: 0,
            display_area_y: 0,

            textured_rectangle_x_flip: false,
            textured_rectangle_y_flip: false,
        }
    }

    pub fn render_frame(&mut self) {
        self.display.draw(&self.vram);
        self.display.handle_events();
    }

    pub fn gpuread(&mut self) -> u32 {
        if self.gpu_to_cpu_transfer.active {
            let lo = self.vram_read_transfer() as u32;
            let hi = self.vram_read_transfer() as u32;

            return (hi << 16) | lo;
        }

        0
    }

    pub fn gpustat(&mut self) -> u32 {
        self.status.read()
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

        let destination_address = Gpu::vram_address(x, y);
        LittleEndian::read_u16(&self.vram[destination_address..])
    }

    fn vram_write_transfer(&mut self, data: u16) {
        let x = self.cpu_to_gpu_transfer.x + self.cpu_to_gpu_transfer.rx;
        let y = self.cpu_to_gpu_transfer.y + self.cpu_to_gpu_transfer.ry;

        let destination_address = Gpu::vram_address(x, y);
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
            self.status.cmd_ready = false;
        }

        if self.command_words_remaining == 0 {
            let command = command_word >> 24;

            self.command_words_remaining = match command {
                0x00 => 1,
                0x01 => 1,
                0x28 => 5,
                0x2c => 9,
                0x30 => 6,
                0x38 => 8,
                0xa0 => 3,
                0xc0 => 3,
                0xe1 => 1,
                0xe2 => 1,
                0xe3 => 1,
                0xe4 => 1,
                0xe5 => 1,
                0xe6 => 1,
                _ => panic!("[GPU] [ERROR] Unknown command GP0({:02x})", command),
            };
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
            0x28 => {
                let coord1 = self.command_buffer.pop();
                let coord2 = self.command_buffer.pop();
                let coord3 = self.command_buffer.pop();
                let coord4 = self.command_buffer.pop();

                let vector1 = Vector2f::new((coord1 & 0xffff) as f32, (coord1 >> 16) as f32);
                let vector2 = Vector2f::new((coord2 & 0xffff) as f32, (coord2 >> 16) as f32);
                let vector3 = Vector2f::new((coord3 & 0xffff) as f32, (coord3 >> 16) as f32);
                let vector4 = Vector2f::new((coord4 & 0xffff) as f32, (coord4 >> 16) as f32);

                let colour = Colour::from_u32(command_word);
    
                let texcoord = Vector2f::new(0.0, 0.0);

                let vertex1 = Vertex::new(vector1, texcoord, colour);
                let vertex2 = Vertex::new(vector2, texcoord, colour);
                let vertex3 = Vertex::new(vector3, texcoord, colour);
                let vertex4 = Vertex::new(vector4, texcoord, colour);

                let quad = Quad::new(vertex1, vertex2, vertex3, vertex4);

                self.rasterise_quad_flat(quad, colour);
            },
            0x2c => {
                use self::TexturePageColours::*;

                let coord1 = self.command_buffer.pop();
                let texcoord1 = self.command_buffer.pop();
                let coord2 = self.command_buffer.pop();
                let texcoord2 = self.command_buffer.pop();
                let coord3 = self.command_buffer.pop();
                let texcoord3 = self.command_buffer.pop();
                let coord4 = self.command_buffer.pop();
                let texcoord4 = self.command_buffer.pop();

                let texpage = texcoord2 >> 16;

                let tpx = (texpage & 0xf) * 64;
                let tpy = ((texpage & 0x10) >> 4) * 256;

                let tpc = match (texpage & 0x180) >> 7 {
                    0 => TP4Bit,
                    1 => TP8Bit,
                    2 => TP15Bit,
                    3 => Reserved,
                    _ => unreachable!(),   
                };

                let clut = texcoord1 >> 16;

                let clut_x = (clut & 0x3f) * 16;
                let clut_y = (clut >> 6) & 0x1ff;

                let tc1 = Vector2f::new((texcoord1 & 0xff) as f32, ((texcoord1 >> 8) & 0xff) as f32);
                let tc2 = Vector2f::new((texcoord2 & 0xff) as f32, ((texcoord2 >> 8) & 0xff) as f32);
                let tc3 = Vector2f::new((texcoord3 & 0xff) as f32, ((texcoord3 >> 8) & 0xff) as f32);
                let tc4 = Vector2f::new((texcoord4 & 0xff) as f32, ((texcoord4 >> 8) & 0xff) as f32);

                let vector1 = Vector2f::new((coord1 & 0xffff) as f32, (coord1 >> 16) as f32);
                let vector2 = Vector2f::new((coord2 & 0xffff) as f32, (coord2 >> 16) as f32);
                let vector3 = Vector2f::new((coord3 & 0xffff) as f32, (coord3 >> 16) as f32);
                let vector4 = Vector2f::new((coord4 & 0xffff) as f32, (coord4 >> 16) as f32);

                let colour = Colour::from_u32(command_word);
    
                let vertex1 = Vertex::new(vector1, tc1, colour);
                let vertex2 = Vertex::new(vector2, tc2, colour);
                let vertex3 = Vertex::new(vector3, tc3, colour);
                let vertex4 = Vertex::new(vector4, tc4, colour);

                let quad = Quad::new(vertex1, vertex2, vertex3, vertex4);

                self.rasterise_quad_textured(quad, tpx, tpy, tpc, clut_x, clut_y);
            },
            0x30 => {
                let colour1 = Colour::from_u32(command_word);
                let coord1 = self.command_buffer.pop();

                let colour2 = Colour::from_u32(self.command_buffer.pop());
                let coord2 = self.command_buffer.pop();

                let colour3 = Colour::from_u32(self.command_buffer.pop());
                let coord3 = self.command_buffer.pop();

                let vector1 = Vector2f::new((coord1 & 0xffff) as f32, (coord1 >> 16) as f32);
                let vector2 = Vector2f::new((coord2 & 0xffff) as f32, (coord2 >> 16) as f32);
                let vector3 = Vector2f::new((coord3 & 0xffff) as f32, (coord3 >> 16) as f32);
    
                let texcoord = Vector2f::new(0.0, 0.0);

                let vertex1 = Vertex::new(vector1, texcoord, colour1);
                let vertex2 = Vertex::new(vector2, texcoord, colour2);
                let vertex3 = Vertex::new(vector3, texcoord, colour3);

                let triangle = Triangle::new(vertex1, vertex2, vertex3);

                self.rasterise_triangle_shaded(triangle);
            },
            0x38 => {
                let colour1 = Colour::from_u32(command_word);
                let coord1 = self.command_buffer.pop();

                let colour2 = Colour::from_u32(self.command_buffer.pop());
                let coord2 = self.command_buffer.pop();

                let colour3 = Colour::from_u32(self.command_buffer.pop());
                let coord3 = self.command_buffer.pop();

                let colour4 = Colour::from_u32(self.command_buffer.pop());
                let coord4 = self.command_buffer.pop();

                let vector1 = Vector2f::new((coord1 & 0xffff) as f32, (coord1 >> 16) as f32);
                let vector2 = Vector2f::new((coord2 & 0xffff) as f32, (coord2 >> 16) as f32);
                let vector3 = Vector2f::new((coord3 & 0xffff) as f32, (coord3 >> 16) as f32);
                let vector4 = Vector2f::new((coord4 & 0xffff) as f32, (coord4 >> 16) as f32);    

                let texcoord = Vector2f::new(0.0, 0.0);
    
                let vertex1 = Vertex::new(vector1, texcoord, colour1);
                let vertex2 = Vertex::new(vector2, texcoord, colour2);
                let vertex3 = Vertex::new(vector3, texcoord, colour3);
                let vertex4 = Vertex::new(vector4, texcoord, colour4);

                let quad = Quad::new(vertex1, vertex2, vertex3, vertex4);

                self.rasterise_quad_shaded(quad);
            },
            0xa0 => {
                let destination = self.command_buffer.pop();
                let size = self.command_buffer.pop();

                let x = destination & 0xffff;
                let y = destination >> 16;
                let w = size & 0xffff;
                let h = size >> 16;

                self.cpu_to_gpu_transfer.x = x;
                self.cpu_to_gpu_transfer.y = y;
                self.cpu_to_gpu_transfer.w = w;
                self.cpu_to_gpu_transfer.h = h;

                self.cpu_to_gpu_transfer.rx = 0;
                self.cpu_to_gpu_transfer.ry = 0;

                self.cpu_to_gpu_transfer.active = true;
            },
            0xc0 => {
                let destination = self.command_buffer.pop();
                let size = self.command_buffer.pop();

                let x = destination & 0xffff;
                let y = destination >> 16;
                let w = size & 0xffff;
                let h = size >> 16;

                self.gpu_to_cpu_transfer.x = x;
                self.gpu_to_cpu_transfer.y = y;
                self.gpu_to_cpu_transfer.w = w;
                self.gpu_to_cpu_transfer.h = h;

                self.gpu_to_cpu_transfer.rx = 0;
                self.gpu_to_cpu_transfer.ry = 0;

                self.gpu_to_cpu_transfer.active = true;
            },
            0xe1 => {
                self.textured_rectangle_y_flip = (command_word & 0x2000) != 0;
                self.textured_rectangle_x_flip = (command_word & 0x1000) != 0;
                self.status.texture_disable = (command_word & 0x800) != 0;
                self.status.display_area_enable = (command_word & 0x400) != 0;
                self.status.dither = (command_word & 0x200) != 0;

                self.status.texture_page_colours = match (command_word & 0x180) >> 7 {
                    0 => TexturePageColours::TP4Bit,
                    1 => TexturePageColours::TP8Bit,
                    2 => TexturePageColours::TP15Bit,
                    3 => TexturePageColours::Reserved,
                    _ => unreachable!(),   
                };

                self.status.semi_transparency = match (command_word & 0x60) >> 5 {
                    0 => SemiTransparency::ST00,
                    1 => SemiTransparency::ST01,
                    2 => SemiTransparency::ST10,
                    3 => SemiTransparency::ST11,
                    _ => unreachable!(),   
                };

                self.status.texture_page_y_base = (command_word & 0x10) != 0;

                self.status.texture_page_x_base = match command_word & 0xf {
                    0x0 => TexturePageXBase::TPX0000,
                    0x1 => TexturePageXBase::TPX0001,
                    0x2 => TexturePageXBase::TPX0010,
                    0x3 => TexturePageXBase::TPX0011,
                    0x4 => TexturePageXBase::TPX0100,
                    0x5 => TexturePageXBase::TPX0101,
                    0x6 => TexturePageXBase::TPX0110,
                    0x7 => TexturePageXBase::TPX0111,
                    0x8 => TexturePageXBase::TPX1000,
                    0x9 => TexturePageXBase::TPX1001,
                    0xa => TexturePageXBase::TPX1010,
                    0xb => TexturePageXBase::TPX1011,
                    0xc => TexturePageXBase::TPX1100,
                    0xd => TexturePageXBase::TPX1101,
                    0xe => TexturePageXBase::TPX1110,
                    0xf => TexturePageXBase::TPX1111,
                    _ => unreachable!(),
                };
            },
            0xe2 => {
                self.texture_window_offset_y = ((command_word & 0xf_8000) >> 15) * 8;
                self.texture_window_offset_x = ((command_word & 0x7c00) >> 10) * 8;
                self.texture_window_mask_y = ((command_word & 0x3e0) >> 5) * 8;
                self.texture_window_mask_x = (command_word & 0x1f) * 8;
            },
            0xe3 => {
                self.drawing_y_begin = command_word & 0x7_fc00 >> 10;
                self.drawing_x_begin = command_word & 0x3ff;
            },
            0xe4 => {
                self.drawing_y_end = command_word & 0x7_fc00 >> 10;
                self.drawing_x_end = command_word & 0x3ff;
            },
            0xe5 => {
                self.drawing_y_offset = (command_word & 0x3f_f800) >> 11;
                self.drawing_x_offset = command_word & 0x7ff;
            },
            0xe6 => {
                self.status.draw_pixels = (command_word & 0x2) != 0;
                self.status.set_mask_bit = (command_word & 0x1) != 0;
            },
            _ => panic!("[GPU] [ERROR] Unknown command GP0({:02x})", command),
        }

        //self.status.cmd_ready = true;
    }

    pub fn execute_gp1_command(&mut self, value: u32) {
        let command = value >> 24;

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

                self.textured_rectangle_y_flip = (value & 0x2000) != 0;
                self.textured_rectangle_x_flip = (value & 0x1000) != 0;
                self.status.texture_disable = (value & 0x800) != 0;
                self.status.display_area_enable = (value & 0x400) != 0;
                self.status.dither = (value & 0x200) != 0;

                self.status.texture_page_colours = match (value & 0x180) >> 7 {
                    0 => TexturePageColours::TP4Bit,
                    1 => TexturePageColours::TP8Bit,
                    2 => TexturePageColours::TP15Bit,
                    3 => TexturePageColours::Reserved,
                    _ => unreachable!(),   
                };

                self.status.semi_transparency = match (value & 0x60) >> 5 {
                    0 => SemiTransparency::ST00,
                    1 => SemiTransparency::ST01,
                    2 => SemiTransparency::ST10,
                    3 => SemiTransparency::ST11,
                    _ => unreachable!(),   
                };

                self.status.texture_page_y_base = (value & 0x10) != 0;

                self.status.texture_page_x_base = match value & 0xf {
                    0x0 => TexturePageXBase::TPX0000,
                    0x1 => TexturePageXBase::TPX0001,
                    0x2 => TexturePageXBase::TPX0010,
                    0x3 => TexturePageXBase::TPX0011,
                    0x4 => TexturePageXBase::TPX0100,
                    0x5 => TexturePageXBase::TPX0101,
                    0x6 => TexturePageXBase::TPX0110,
                    0x7 => TexturePageXBase::TPX0111,
                    0x8 => TexturePageXBase::TPX1000,
                    0x9 => TexturePageXBase::TPX1001,
                    0xa => TexturePageXBase::TPX1010,
                    0xb => TexturePageXBase::TPX1011,
                    0xc => TexturePageXBase::TPX1100,
                    0xd => TexturePageXBase::TPX1101,
                    0xe => TexturePageXBase::TPX1110,
                    0xf => TexturePageXBase::TPX1111,
                    _ => unreachable!(),
                };

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

                self.status.draw_pixels = false;
                self.status.set_mask_bit = false;
            },
            0x01 => {
                self.command_buffer.clear();
            },
            0x02 => {
                self.status.irq = false;
            },
            0x03 => {
                self.status.display_disable = (value & 0x1) != 0;
            },
            0x04 => {
                self.status.dma_direction = match value & 0x3 {
                    0 => DmaDirection::Off,
                    1 => DmaDirection::Fifo,
                    2 => DmaDirection::CpuToGp0,
                    3 => DmaDirection::GpureadToCpu,
                    _ => unreachable!(),
                };
            },
            0x05 => {
                self.display_area_x = (value & 0x7_fc00) >> 10;
                self.display_area_y = value & 0x3ff;
            },
            0x06 => {
                // TODO: Horizontal Display Range
            },
            0x07 => {
                // TODO: Vertical Display Range
            },
            0x08 => {
                self.status.reverse = (value & 0x80) != 0;
                self.status.horizontal_resolution_2 = (value & 0x40) != 0;
                self.status.vertical_interlace = (value & 0x20) != 0;

                self.status.display_colour_depth = (value & 0x10) != 0;

                if self.status.display_colour_depth {
                    panic!("[GPU] [ERROR] Unsupported display colour depth: 24bit.");
                }

                self.status.video_mode = (value & 0x8) != 0;
                self.status.vertical_resolution = (value & 0x4) != 0;
                self.status.horizontal_resolution_1 = match value & 0x3 {
                    0 => HorizontalResolution::X256,
                    1 => HorizontalResolution::X320,
                    2 => HorizontalResolution::X512,
                    3 => HorizontalResolution::X640,
                    _ => unreachable!(),
                };

                self.update_video_mode();
            }
            _ => panic!("[GPU] [ERROR] Unknown command GP1({:02x})", command),
        }
    }

    fn rasterise_pixel(&mut self, xcoord: u32, ycoord: u32, colour: Colour) {
        let r = (colour.r * 255.0) as u16;
        let g = (colour.g * 255.0) as u16;
        let b = (colour.b * 255.0) as u16;

        let mut pixel: u16 = 0;
        pixel |= (r & 0xf8) << 7;
        pixel |= (g & 0xf8) << 2;
        pixel |= (b & 0xf8) >> 3;

        let x = (self.drawing_x_offset + xcoord) as usize;
        let y = (self.drawing_x_offset + ycoord) as usize;

        self.vram[2 * (x + y * 1024)] = pixel as u8;
        self.vram[2 * (x + y * 1024) + 1] = (pixel >> 8) as u8;
    }

    fn rasterise_triangle_flat(&mut self, triangle: Triangle, colour: Colour) {
        let (min, max) = triangle.bounding_box();

        let mut y = min.y;

        while y < max.y {
            let mut x = min.x;

            while x < max.x {
                let v = triangle.barycentric_vector(x, y);

                let vx = v.x.floor() as isize;
                let vy = v.y.floor() as isize;
                let vz = v.z.floor() as isize;

                if (vx | vy | vz) >= 0 {
                    self.rasterise_pixel(x as u32, y as u32, colour);
                }

                x += 1.0;
            }

            y += 1.0;
        }
    }

    fn rasterise_triangle_shaded(&mut self, triangle: Triangle) {
        let (min, max) = triangle.bounding_box();

        let mut y = min.y;

        while y < max.y {
            let mut x = min.x;

            while x < max.x {
                let v = triangle.barycentric_vector(x, y);

                let vx = v.x.floor() as isize;
                let vy = v.y.floor() as isize;
                let vz = v.z.floor() as isize;

                if (vx | vy | vz) >= 0 {
                    let colour = triangle.interpolate_colour(v);
                    self.rasterise_pixel(x as u32, y as u32, colour);
                }

                x += 1.0;
            }

            y += 1.0;
        }
    }

    fn rasterise_triangle_textured(&mut self, triangle: Triangle, tpx: u32, tpy: u32, tpc: TexturePageColours, clut_x: u32, clut_y: u32) {
        let (min, max) = triangle.bounding_box();

        let mut y = min.y;

        while y < max.y {
            let mut x = min.x;

            while x < max.x {
                let v = triangle.barycentric_vector(x, y);

                let vx = v.x.floor() as isize;
                let vy = v.y.floor() as isize;
                let vz = v.z.floor() as isize;

                if (vx | vy | vz) >= 0 {
                    let texcoord = triangle.interpolate_texcoord(v);

                    let mut draw_pixel = true;

                    let texture_colour;

                    match tpc {
                        TexturePageColours::TP4Bit => {
                            let texture_pixel = self.read_clut_4bit(texcoord.x as u32, texcoord.y as u32, tpx, tpy, clut_x, clut_y);

                            if texture_pixel == 0 {
                                draw_pixel = false;
                            }

                            texture_colour = Colour::from_u16_bgr(texture_pixel);
                        },
                        TexturePageColours::TP8Bit => panic!("Unsupported texture bit-depth: 8-bit CLUT"),
                        TexturePageColours::TP15Bit | TexturePageColours::Reserved => {
                            let texture_pixel = self.read_texpage(texcoord.x as u32, texcoord.y as u32, tpx, tpy);
                            texture_colour = Colour::from_u16(texture_pixel);
                        },
                    };

                    if draw_pixel {
                        self.rasterise_pixel(x as u32, y as u32, texture_colour);
                    }
                }

                x += 1.0;
            }

            y += 1.0;
        }
    }

    fn read_texpage(&self, texcoord_x: u32, texcoord_y: u32, tpx: u32, tpy: u32) -> u16 {
        let x = tpx + texcoord_x;
        let y = tpy + texcoord_y;

        let address = 2 * (x + y * 1024);

        LittleEndian::read_u16(&self.vram[address as usize..])
    }

    fn read_clut_4bit(&self, texcoord_x: u32, texcoord_y: u32, tpx: u32, tpy: u32, clut_x: u32, clut_y: u32) -> u16 {
        let texture_address = (2 * tpx + texcoord_x / 2 + (tpy + texcoord_y) * 2048) as usize;
        let mut clut_index = self.vram[texture_address] as usize;

        if texcoord_x & 0x1 != 0 {
            clut_index >>= 4;
        } else {
            clut_index &= 0x0f;
        }

        let clut_address = 2 * (clut_x + clut_y * 1024) as usize;
        let indexed_clut = clut_address + clut_index * 2;

        LittleEndian::read_u16(&self.vram[indexed_clut..])
    }

    fn rasterise_quad_flat(&mut self, quad: Quad, colour: Colour) {
        let a = quad.vertices[0];
        let b = quad.vertices[1];
        let c = quad.vertices[2];
        let d = quad.vertices[3];

        let t1 = Triangle::new(a, b, c);
        let t2 = Triangle::new(b, c, d);

        self.rasterise_triangle_flat(t1, colour);
        self.rasterise_triangle_flat(t2, colour);
    }

    fn rasterise_quad_shaded(&mut self, quad: Quad) {
        let a = quad.vertices[0];
        let b = quad.vertices[1];
        let c = quad.vertices[2];
        let d = quad.vertices[3];

        let t1 = Triangle::new(a, b, c);
        let t2 = Triangle::new(b, c, d);

        self.rasterise_triangle_shaded(t1);
        self.rasterise_triangle_shaded(t2);
    }

    fn rasterise_quad_textured(&mut self, quad: Quad, tpx: u32, tpy: u32, tpc: TexturePageColours, clut_x: u32, clut_y: u32) {
        let a = quad.vertices[0];
        let b = quad.vertices[1];
        let c = quad.vertices[2];
        let d = quad.vertices[3];

        let t1 = Triangle::new(a, b, c);
        let t2 = Triangle::new(b, c, d);

        self.rasterise_triangle_textured(t1, tpx, tpy, tpc, clut_x, clut_y);
        self.rasterise_triangle_textured(t2, tpx, tpy, tpc, clut_x, clut_y);
    }

    pub fn update_video_mode(&mut self) {
        let h1 = self.status.horizontal_resolution_1;
        let h2 = self.status.horizontal_resolution_2;

        let v = self.status.vertical_resolution;
        let i = self.status.vertical_interlace;

        let width = match (h1, h2) {
            (_, true) => 368,
            (HorizontalResolution::X256, false) => 256,
            (HorizontalResolution::X320, false) => 320,
            (HorizontalResolution::X512, false) => 512,
            (HorizontalResolution::X640, false) => 640,
        };

        let height = match (v, i) {
            (true, true) => 480,
            (_, _) => 240,
        };

        self.display.update_video_mode(width, height);
    }
}