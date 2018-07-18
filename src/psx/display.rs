use byteorder::{LittleEndian, ByteOrder};
use sdl2;
use sdl2::event::Event;
use sdl2::EventPump;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::render::{TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;

use std::cell::RefCell;
use std::rc::Rc;
//use std::thread;
//use std::time::{Duration, Instant};

use super::controller::Controller;
use super::rasteriser::Colour;

//ub const FRAME_TIME: f64 = 1.0 / 59.29;

struct SdlContext {
    canvas: WindowCanvas,
    event_pump: EventPump,
    texture_creator: TextureCreator<WindowContext>,
}

impl SdlContext {
    pub fn new(width: u32, height: u32, title: &str) -> SdlContext {
        let context = sdl2::init().unwrap();
        let event_pump = context.event_pump().unwrap();
        let video_subsystem = context.video().unwrap();
        let window = video_subsystem.window(title, width, height).build().unwrap();
        let canvas = window.into_canvas().build().unwrap();
        let texture_creator = canvas.texture_creator();

        SdlContext {
            canvas: canvas,
            event_pump: event_pump,
            texture_creator: texture_creator,
        }
    }

    pub fn draw(&mut self, framebuffer: &[u8], gpu_width: u32, gpu_height: u32) {
        let (w, h) = self.canvas.window().size();

        let gpu_w = gpu_width as usize;
        let gpu_h = gpu_height as usize;

        let mut texture = self.texture_creator.create_texture_streaming(PixelFormatEnum::RGB24, gpu_width, gpu_height).unwrap();

        self.canvas.clear();

        texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
            for y in 0..gpu_h {
                for x in 0..gpu_w {
                    let texture_address = (y * pitch) + (x * 3);

                    let framebuffer_address = 2 * (x + y * 1024);

                    let pixel = LittleEndian::read_u16(&framebuffer[framebuffer_address..]);
                    let colour = Colour::from_u16(pixel);

                    let (r, g, b, _) = colour.to_u8();

                    buffer[texture_address]     = r;
                    buffer[texture_address + 1] = g;
                    buffer[texture_address + 2] = b;
                }
            }
        }).unwrap();

        self.canvas.copy(&texture, None, Some(Rect::new(0, 0, w, h))).unwrap();
		self.canvas.present();
    }

    pub fn handle_events(&mut self, controller: &mut Controller) {
        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit {..} => {
                    panic!();
                },

                Event::KeyDown {keycode, ..} => {
					controller.set(keycode.unwrap(), false);
				},

				Event::KeyUp {keycode, ..} => {
					controller.set(keycode.unwrap(), true);
				},

                _ => (),
            }
        }
    }
}

pub struct Display {
    gpu_width: u32,
    gpu_height: u32,

    controller: Rc<RefCell<Controller>>,

    sdl_context: SdlContext,

    //last_frame_time: Instant,
}

impl Display {
    pub fn new(width: u32, height: u32, title: &str, controller: Rc<RefCell<Controller>>) -> Display {
        Display {
            gpu_width: 256,
            gpu_height: 240,
            
            controller: controller,

            sdl_context: SdlContext::new(width, height, title),
            
            //last_frame_time: Instant::now(),
        }
    }

    pub fn draw(&mut self, framebuffer: &[u8]) {
        self.sdl_context.draw(framebuffer, self.gpu_width, self.gpu_height);

        //let elapsed = self.last_frame_time.elapsed();
		//let elapsed_ms = (elapsed.as_secs() as f64 * 1000.0) + (elapsed.subsec_nanos() as f64 / 1000000.0);

        //if elapsed_ms < FRAME_TIME {
		//    let sleep_time = (FRAME_TIME - elapsed_ms) as u64;
        //
        //    if sleep_time != 0 {
		//		thread::sleep(Duration::from_millis(sleep_time));
        //        println!("slep");
		//	}
        //}

        //self.last_frame_time = Instant::now();
    }

    pub fn handle_events(&mut self) {
        let mut controller = self.controller.borrow_mut();

        self.sdl_context.handle_events(&mut controller);
    }

    pub fn update_video_mode(&mut self, width: u32, height: u32) {
        if self.gpu_width == width && self.gpu_height == height {
            return;
        }

        println!("[DISPLAY] [INFO] Changing display mode to {}x{}", width, height);

        self.gpu_width = width;
        self.gpu_height = height;
    }
}