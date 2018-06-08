use byteorder::{LittleEndian, ByteOrder};

use super::display::Display;
use super::rasteriser::{Colour, Quad, Triangle, Vector2f, Vertex};

pub const DITHER_TABLE: [isize; 16] = [-4,  0, -3,  1,
                                        2, -2,  3, -1,
                                       -3,  1, -4,  0,
                                        3, -1,  2, -2];

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
enum TexturePageColours {
    TP4Bit,
    TP8Bit,
    TP15Bit,
    Reserved,
}

#[derive(Clone, Copy)]
enum SemiTransparency {
    ST0,
    ST1,
    ST2,
    ST3,
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
            semi_transparency: SemiTransparency::ST0,
            y_base: 0,
            x_base: 0,
        }
    }

    pub fn from_u32(value: u32) -> GpuTexpage {
        GpuTexpage {
            flip_y: (value & 0x2000) != 0,
            flip_x: (value & 0x1000) != 0,
            texture_disable: (value & 0x800) != 0,
            display_area_enable: (value & 0x400) != 0,
            dithering_enable: (value & 0x200) != 0,
            colour_depth: match (value & 0x180) >> 7 {
                0 => TexturePageColours::TP4Bit,
                1 => TexturePageColours::TP8Bit,
                2 => TexturePageColours::TP15Bit,
                3 => TexturePageColours::Reserved,
                _ => unreachable!(),
            },
            semi_transparency: match (value & 0x60) >> 5 {
                0 => SemiTransparency::ST0,
                1 => SemiTransparency::ST1,
                2 => SemiTransparency::ST2,
                3 => SemiTransparency::ST3,
                _ => unreachable!(),
            },
            y_base: (value & 0x10) * 16,
            x_base: (value & 0xf) * 64,
        }
    }
}

pub struct Gpu {
    display: Display,

    vram: Box<[u8]>,

    command_buffer: GpuQueue,
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

    vertical_resolution: u32,
    horizontal_resolution: u32,

    reverse: bool,

    skip_masked_pixels: bool,
    set_mask_bit: bool,

    texpage: GpuTexpage,

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

            vertical_resolution: 240,
            horizontal_resolution: 256,

            reverse: false,

            skip_masked_pixels: false,
            set_mask_bit: false,

            texpage: GpuTexpage::new(),

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
        }
    }

    fn dither_get_offset(x: isize, y: isize) -> isize {
        let xi = x & 0x3;
        let yi = y & 0x3;

        let index = (xi + yi * 4) as usize;
        DITHER_TABLE[index]
    }

    fn dither_saturating_add(v: u8, d: isize) -> u8 {
        let r = (v as isize) + d;

        if r < 0 {
            return 0;
        }

        if r > 0xff {
            return 0xff;
        }

        r as u8
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
        value |= match self.vertical_resolution {
            480 => (1 << 19),
            240 => 0,
            _ => unreachable!(),
        };
        value |= match self.horizontal_resolution {
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
            self.cmd_ready = false;
        }

        if self.command_words_remaining == 0 {
            let command = command_word >> 24;

            self.command_words_remaining = match command {
                0x00 => 1,
                0x01 => 1,
                0x02 => 3,
                0x28 => 5,
                0x2c => 9,
                0x2d => 9,
                0x30 => 6,
                0x38 => 8,
                0x64 => 4,
                0x65 => 4,
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
            0x02 => {
                let destination = self.command_buffer.pop();
                let size = self.command_buffer.pop();

                let colour = Colour::from_u32(command_word);
                let pixel = colour.to_u16();

                let x_start = destination & 0xfff0;
                let y_start = destination >> 16;
                
                let w = (size) & 0xfff0;
                let h = size >> 16;

                let x_end = x_start + w;
                let y_end = y_start + h;

                for y in y_start..y_end {
                    for x in x_start..x_end + 0x10 {
                        let destination_address = Gpu::vram_address(x, y);
                        LittleEndian::write_u16(&mut self.vram[destination_address..], pixel);
                    }
                }
            }
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
            0x2c | 0x2d => {
                let coord1 = self.command_buffer.pop();
                let texcoord1 = self.command_buffer.pop();
                let coord2 = self.command_buffer.pop();
                let texcoord2 = self.command_buffer.pop();
                let coord3 = self.command_buffer.pop();
                let texcoord3 = self.command_buffer.pop();
                let coord4 = self.command_buffer.pop();
                let texcoord4 = self.command_buffer.pop();

                let texpage = GpuTexpage::from_u32(texcoord2 >> 16);

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

                let dither = match command {
                    0x2c => true,
                    0x2d => false,
                    _ => unreachable!(),
                };

                self.rasterise_quad_textured(quad, texpage, clut_x, clut_y, dither);
            },
            0x30 => {
                let colour1 = Colour::from_u32(command_word);
                let coord1 = self.command_buffer.pop() as i32;

                let colour2 = Colour::from_u32(self.command_buffer.pop());
                let coord2 = self.command_buffer.pop() as i32;

                let colour3 = Colour::from_u32(self.command_buffer.pop());
                let coord3 = self.command_buffer.pop() as i32;

                let mut vector1 = Vector2f::new((coord1 & 0xffff) as f32, (coord1 >> 16) as f32);
                let mut vector2 = Vector2f::new((coord2 & 0xffff) as f32, (coord2 >> 16) as f32);
                let mut vector3 = Vector2f::new((coord3 & 0xffff) as f32, (coord3 >> 16) as f32);

                let texcoord = Vector2f::new(0.0, 0.0);

                let vertex1 = Vertex::new(vector1, texcoord, colour1);
                let vertex2 = Vertex::new(vector2, texcoord, colour2);
                let vertex3 = Vertex::new(vector3, texcoord, colour3);

                let triangle = Triangle::new(vertex1, vertex2, vertex3);

                self.rasterise_triangle_shaded(triangle, true);
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

                self.rasterise_quad_shaded(quad, true);
            },
            0x64 | 0x65 => {
                let colour = Colour::from_u32(command_word);

                let coord = self.command_buffer.pop();
                let x = coord & 0xffff;
                let y = coord >> 16;

                let texcoord = self.command_buffer.pop();

                let tx = texcoord & 0xff;
                let ty = (texcoord >> 8) & 0xff;

                let clut = texcoord >> 16;
                let clut_x = (clut & 0x3f) * 16;
                let clut_y = (clut >> 6) & 0x1ff;

                let size = self.command_buffer.pop();
                let w = size & 0xffff;
                let h = size >> 16;

                let vector1 = Vector2f::new(x as f32, y as f32);
                let vector2 = Vector2f::new((x + w) as f32, y as f32);
                let vector3 = Vector2f::new(x as f32, (y + h) as f32);
                let vector4 = Vector2f::new((x + w) as f32, (y + h) as f32);

                let tc1 = Vector2f::new(tx as f32, ty as f32);
                let tc2 = Vector2f::new(((tx + w) & 0xff) as f32, ty as f32);
                let tc3 = Vector2f::new(tx as f32, ((ty + h) & 0xff) as f32);
                let tc4 = Vector2f::new(((tx + w) & 0xff) as f32, ((ty + h) & 0xff) as f32);

                let vertex1 = Vertex::new(vector1, tc1, colour);
                let vertex2 = Vertex::new(vector2, tc2, colour);
                let vertex3 = Vertex::new(vector3, tc3, colour);
                let vertex4 = Vertex::new(vector4, tc4, colour);

                let quad = Quad::new(vertex1, vertex2, vertex3, vertex4);

                let texpage = self.texpage;
                self.rasterise_quad_textured(quad, texpage, clut_x, clut_y, false);
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
                    0 => SemiTransparency::ST0,
                    1 => SemiTransparency::ST1,
                    2 => SemiTransparency::ST2,
                    3 => SemiTransparency::ST3,
                    _ => unreachable!(),   
                };

                self.texpage.y_base = (command_word & 0x10) * 16;
                self.texpage.x_base = (command_word & 0xf) * 64;
            },
            0xe2 => {
                self.texture_window_offset_y = ((command_word & 0xf_8000) >> 15) * 8;
                self.texture_window_offset_x = ((command_word & 0x7c00) >> 10) * 8;
                self.texture_window_mask_y = ((command_word & 0x3e0) >> 5) * 8;
                self.texture_window_mask_x = (command_word & 0x1f) * 8;
            },
            0xe3 => {
                self.drawing_y_begin = (command_word & 0x7_fc00) >> 10;
                self.drawing_x_begin = command_word & 0x3ff;
            },
            0xe4 => {
                self.drawing_y_end = (command_word & 0x7_fc00) >> 10;
                self.drawing_x_end = command_word & 0x3ff;
            },
            0xe5 => {
                self.drawing_y_offset = (command_word & 0x3f_f800) >> 11;
                self.drawing_x_offset = command_word & 0x7ff;
            },
            0xe6 => {
                self.skip_masked_pixels = (command_word & 0x2) != 0;
                self.set_mask_bit = (command_word & 0x1) != 0;
            },
            _ => panic!("[GPU] [ERROR] Unknown command GP0({:02x})", command),
        }
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

                self.texpage.flip_y = false;
                self.texpage.flip_x = false;
                self.texpage.texture_disable = false;
                self.texpage.display_area_enable = false;
                self.texpage.dithering_enable = false;
                self.texpage.colour_depth =TexturePageColours::TP4Bit;
                self.texpage.semi_transparency = SemiTransparency::ST0;
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
                self.display_disable = (value & 0x1) != 0;
            },
            0x04 => {
                self.dma_direction = match value & 0x3 {
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
                self.reverse = (value & 0x80) != 0;
                self.vertical_interlace = (value & 0x20) != 0;
                self.colour_depth = (value & 0x10) != 0;

                if self.colour_depth {
                    panic!("[GPU] [ERROR] Unsupported display colour depth: 24bit.");
                }

                self.video_mode = (value & 0x8) != 0;

                self.vertical_resolution = match (self.vertical_interlace, (value & 0x4) != 0) {
                    (true, true) => 480,
                    _ => 240,
                };

                self.horizontal_resolution = match ((value & 0x40) != 0, value & 0x3) {
                    (true, _) => 368,
                    (false, 0) => 256,
                    (false, 1) => 320,
                    (false, 2) => 512,
                    (false, 3) => 640,
                    _ => unreachable!(),
                };

                self.update_video_mode();
            }
            _ => panic!("[GPU] [ERROR] Unknown command GP1({:02x})", command),
        }
    }

    fn rasterise_pixel(&mut self, x_coord: isize, y_coord: isize, colour: Colour, dither: bool) {
        if x_coord < 0 || x_coord >= 1024 || y_coord < 0 || y_coord >= 512 {
            return;
        }

        if (x_coord as u32) < self.drawing_x_begin {
            return;
        }

        if (x_coord as u32) > self.drawing_x_end {
            return;
        }

        if (y_coord as u32) < self.drawing_y_begin {
            return;
        }

        if (y_coord as u32) > self.drawing_y_end {
            return;
        }

        let (mut r, mut g, mut b) = colour.to_u8();

        let pixel;

        if dither && self.texpage.dithering_enable {
            let dither_offset = Gpu::dither_get_offset(x_coord, y_coord);
            r = Gpu::dither_saturating_add(r, dither_offset);
            g = Gpu::dither_saturating_add(g, dither_offset);
            b = Gpu::dither_saturating_add(b, dither_offset);
            
            pixel = Colour::from_u8(r, g, b).to_u16();
        } else {
            pixel = colour.to_u16();
        }

        let x = x_coord as usize;
        let y = y_coord as usize;

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

                let vx = v.x.round() as isize;
                let vy = v.y.round() as isize;
                let vz = v.z.round() as isize;

                if (vx | vy | vz) >= 0 {
                    self.rasterise_pixel(x as isize, y as isize, colour, false);
                }

                x += 1.0;
            }

            y += 1.0;
        }
    }

    fn rasterise_triangle_shaded(&mut self, triangle: Triangle, dither: bool) {
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
                    self.rasterise_pixel(x as isize, y as isize, colour, dither);
                }

                x += 1.0;
            }

            y += 1.0;
        }
    }

    fn rasterise_triangle_textured(&mut self, triangle: Triangle, texpage: GpuTexpage, clut_x: u32, clut_y: u32, dither: bool) {
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

                    match texpage.colour_depth {
                        TexturePageColours::TP4Bit => {
                            let texture_pixel = self.read_clut_4bit(texcoord.x as u32, texcoord.y as u32, texpage.x_base, texpage.y_base, clut_x, clut_y);

                            if texture_pixel == 0 {
                                draw_pixel = false;
                            }

                            texture_colour = Colour::from_u16(texture_pixel);
                        },
                        TexturePageColours::TP8Bit => panic!("Unsupported texture bit-depth: 8-bit CLUT"),
                        TexturePageColours::TP15Bit | TexturePageColours::Reserved => {
                            let texture_pixel = self.read_texpage(texcoord.x as u32, texcoord.y as u32, texpage.x_base, texpage.y_base);

                            if texture_pixel == 0 {
                                draw_pixel = false;
                            }

                            texture_colour = Colour::from_u16(texture_pixel);
                        },
                    };

                    if draw_pixel {
                        self.rasterise_pixel(x as isize, y as isize, texture_colour, dither);
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

        LittleEndian::read_u16(&self.vram[Gpu::vram_address(x, y)..])
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

    fn rasterise_quad_shaded(&mut self, quad: Quad, dither: bool) {
        let a = quad.vertices[0];
        let b = quad.vertices[1];
        let c = quad.vertices[2];
        let d = quad.vertices[3];

        let t1 = Triangle::new(a, b, c);
        let t2 = Triangle::new(b, c, d);

        self.rasterise_triangle_shaded(t1, dither);
        self.rasterise_triangle_shaded(t2, dither);
    }

    fn rasterise_quad_textured(&mut self, quad: Quad, texpage: GpuTexpage, clut_x: u32, clut_y: u32, dither: bool) {
        let a = quad.vertices[0];
        let b = quad.vertices[1];
        let c = quad.vertices[2];
        let d = quad.vertices[3];

        let t1 = Triangle::new(a, b, c);
        let t2 = Triangle::new(b, c, d);

        self.rasterise_triangle_textured(t1, texpage, clut_x, clut_y, dither);
        self.rasterise_triangle_textured(t2, texpage, clut_x, clut_y, dither);
    }

    pub fn update_video_mode(&mut self) {
        self.display.update_video_mode(self.horizontal_resolution, self.vertical_resolution);
    }
}