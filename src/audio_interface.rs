use std::collections::VecDeque;
use std::ops::DerefMut;

use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};

struct AudioBuffer {
    data: VecDeque<i16>,
}

impl AudioBuffer {
    pub fn new() -> AudioBuffer {
        AudioBuffer {
            data: VecDeque::new(),
        }
    }

    pub fn push_samples(&mut self, samples: Vec<i16>) {
        for sample in samples.iter() {
            self.data.push_back(*sample);
        }

        while self.data.len() > 512 * 16 {
            self.data.pop_front().unwrap();
        }
    }
}

impl AudioCallback for AudioBuffer {
    type Channel = i16;

    fn callback(&mut self, out: &mut [i16]) {
        let len = self.data.len();

        let (last_l, last_r) = if len >= 2 {
            (self.data[len - 2], self.data[len - 1])
        } else {
            (0, 0)
        };

        for i in 0..out.len() {
            if let Some(s) = self.data.pop_front() {
                out[i] = s;
            } else if (i % 2) == 0 {
                out[i] = last_l;
            } else {
                out[i] = last_r;
            }
        }
    }
}

pub struct AudioInterface {
    device: AudioDevice<AudioBuffer>,
}

impl AudioInterface {
    pub fn new(ctx_tmp: &mut sdl2::Sdl, freq: i32, channels: u8, samples: u16) -> AudioInterface {
        let audio_subsystem = ctx_tmp.audio().unwrap();

        let desired_spec = AudioSpecDesired {
            freq: Some(freq),
            channels: Some(channels),
            samples: Some(samples),
        };

        let device = audio_subsystem
            .open_playback(None, &desired_spec, |_| AudioBuffer::new())
            .unwrap();

        AudioInterface { device: device }
    }

    pub fn play(&mut self) {
        self.device.resume();
    }

    pub fn push_samples(&mut self, samples: Vec<i16>) {
        self.device.lock().deref_mut().push_samples(samples);
    }
}
