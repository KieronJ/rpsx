extern crate byteorder;
extern crate sdl2;

//mod debugger;
mod psx;
pub mod queue;
pub mod util;

use std::env;

//use debugger::Debugger;
use psx::System;

fn main() {
    let bios_filepath = match env::args().nth(1) {
        Some(x) => x,
        None => {
            println!("usage: rpsx.exe rom game");
            return;
        }
    };

    let game_filepath = match env::args().nth(2) {
        Some(x) => x,
        None => {
            println!("usage: rpsx.exe rom game");
            return;
        }
    };
    
    //let system = System::new(&bios_filepath, &game_filepath);
    //let mut debugger = Debugger::new(system);
	//debugger.reset();
    //debugger.run();

    let mut system = System::new(&bios_filepath, &game_filepath);
    system.reset();

    loop {
        for _ in 0..7 {
            system.run();
        }
    
        system.tick(7);
        system.check_dma_int();
        system.tick_gpu(22);
    }
}