extern crate byteorder;
extern crate sdl2;

mod debugger;
mod psx;
pub mod util;

use std::env;
use std::process;

//use debugger::Debugger;
use psx::interrupt::Interrupt;
use psx::System;

fn main()
{
    let bios = match env::args().nth(1) {
        Some(x) => x,
        None => {
            println!("usage: rpsx.exe rom");
            process::exit(1);
        }
    };

    //let system = System::new(&bios);
    //let mut debugger = Debugger::new(system);
	//debugger.reset();
    //debugger.run();

    let mut system = System::new(&bios);
    system.reset();
    
    loop {
        for _ in 0..285620 {
            system.run();
    
            for _ in 0..2 {
                system.tick();
            }
        }
    
        system.render_frame();
        system.set_interrupt(Interrupt::Vblank);
    }
}