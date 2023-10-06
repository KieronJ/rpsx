#[macro_use]
extern crate clap;

extern crate imgui;

mod audio_interface;
mod frontend;
//mod gui;

mod psx;
pub mod queue;
pub mod util;

use clap::App;

use audio_interface::AudioInterface;
use frontend::Frontend;
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

pub struct Options {
    draw_full_vram: bool,
    scaling: Scaling,
    crop_overscan: bool,

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
        scaling: Scaling::Aspect,
        crop_overscan: true,

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

    let mut system = System::new(bios_filepath.to_string(), game_filepath.to_string());
    system.reset();

    audio.play();

    while system.running {
        if options.step {
            system.run_frame();

            options.step = false;
            options.pause = true;
        }

        if !options.pause {
            system.run_frame();
        }

        audio.push_samples(system.get_audio_samples());
        frontend.update(&mut options, &mut system);
        frontend.render(&options, &system);
    }
}
