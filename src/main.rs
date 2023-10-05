#[macro_use]
extern crate clap;

extern crate imgui;

mod audio_interface;
mod frontend;
mod gpu_viewer;
//mod gui;

mod psx;
pub mod queue;
pub mod util;

use clap::App;

use audio_interface::AudioInterface;
use frontend::Frontend;
//use gpu_viewer::GpuFrame;
//use gui::Gui;

use psx::System;

#[derive(Clone, Copy)]
pub enum Scaling {
    None,
    Aspect,
    Fullscreen,
}

impl Scaling {
    pub fn from(value: i32) -> Scaling {
        use Scaling::*;

        match value {
            0 => None,
            1 => Aspect,
            2 => Fullscreen,
            _ => panic!(),
        }
    }
}

pub struct GpuViewerOptions {
    overlay_position: bool,
    overlay_texture: bool,
    overlay_clut: bool,
}

pub struct Options {
    draw_full_vram: bool,
    draw_display_area: bool,

    show_gpu_viewer: bool,
    show_metrics: bool,

    gpu_viewer: GpuViewerOptions,

    scaling: Scaling,

    pause: bool,
    step: bool,

    frame_limit: bool,

    state_index: usize,
}

fn main() {
    let yaml = load_yaml!("../cli.yaml");
    let matches = App::from_yaml(yaml).get_matches();

    let bios_filepath = matches.value_of("BIOS").unwrap();
    let game_filepath = matches.value_of("GAME").unwrap();

    let mut options = Options {
        draw_full_vram: false,
        draw_display_area: false,

        show_gpu_viewer: false,
        show_metrics: false,

        gpu_viewer: GpuViewerOptions {
            overlay_position: true,
            overlay_texture: true,
            overlay_clut: true,
        },

        scaling: Scaling::Aspect,

        pause: false,
        step: false,

        frame_limit: true,

        state_index: 0,
    };

    let mut sdl_ctx_temp = sdl2::init().unwrap();
    let mut audio = AudioInterface::new(&mut sdl_ctx_temp, 44100, 2, 512);
    let mut frontend = Frontend::create(&mut sdl_ctx_temp, 640, 480);

    // Disabled due to Dear ImGui version bump
    //let mut gui = Gui::new(&video.display);
    //let mut gpu_frame = GpuFrame::new();

    let mut system = System::new(bios_filepath.to_string(), game_filepath.to_string());
    system.reset();

    audio.play();

    while system.running {
        if options.step {
            system.run_frame();
            //gpu_frame.take(system.get_frame_data());

            options.step = false;
            options.pause = true;
        }

        if !options.pause {
            system.run_frame();
            //gpu_frame.take(system.get_frame_data());
        }

        audio.push_samples(system.get_audio_samples());
        frontend.update(&mut options, &mut system);
        frontend.render(&options, &system);
    }
}
