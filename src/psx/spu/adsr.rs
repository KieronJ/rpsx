use std::cmp;

use serde::{Deserialize, Serialize};

use crate::util::clip;

#[derive(Clone, Copy, Deserialize, PartialEq, Serialize)]
pub enum AdsrState {
    Disabled,
    Attack,
    Decay,
    Sustain,
    Release,
}

impl Default for AdsrState {
    fn default() -> AdsrState {
        AdsrState::Disabled
    }
}

#[derive(PartialEq)]
pub enum AdsrMode {
    Linear,
    Exponential,
}

#[derive(PartialEq)]
pub enum AdsrDirection {
    Increase,
    Decrease,
}

/*
 *  TODO: Perhaps encode this information when writing the u32 config
 *  register this would make it much neater, whilst also probably being faster
 */
struct AdsrConfig {
    pub mode: AdsrMode,
    pub direction: AdsrDirection,
    pub shift: isize,
    pub step: isize,
    pub target: isize,
    pub next: AdsrState,
}

impl AdsrConfig {
    fn attack_mode(config: u32) -> AdsrMode {
        match (config & 0x8000) != 0 {
            false => AdsrMode::Linear,
            true => AdsrMode::Exponential,
        }
    }

    fn attack_shift(config: u32) -> isize {
        ((config & 0x7c00) >> 10) as isize
    }

    fn attack_step(config: u32) -> isize {
        let step = ((config & 0x300) >> 8) as isize;

        7 - step
    }

    fn decay_shift(config: u32) -> isize {
        ((config & 0xf0) >> 4) as isize
    }

    fn sustain_mode(config: u32) -> AdsrMode {
        match (config & 0x8000_0000) != 0 {
            false => AdsrMode::Linear,
            true => AdsrMode::Exponential,
        }
    }

    fn sustain_direction(config: u32) -> AdsrDirection {
        match (config & 0x4000_0000) != 0 {
            false => AdsrDirection::Increase,
            true => AdsrDirection::Decrease,
        }
    }

    fn sustain_shift(config: u32) -> isize {
        ((config & 0x1f00_0000) >> 24) as isize
    }

    fn sustain_step(config: u32) -> isize {
        let step = ((config & 0xc0_0000) >> 22) as isize;

        match AdsrConfig::sustain_direction(config) {
            AdsrDirection::Increase => 7 - step,
            AdsrDirection::Decrease => -8 + step,
        }
    }

    fn sustain_level(config: u32) -> isize {
        let level = (config & 0xf) as isize;

        (level + 1) * 0x800
    }

    fn release_mode(config: u32) -> AdsrMode {
        match (config & 0x20_0000) != 0 {
            false => AdsrMode::Linear,
            true => AdsrMode::Exponential,
        }
    }

    fn release_shift(config: u32) -> isize {
        ((config & 0x1f_0000) >> 16) as isize
    }

    pub fn from(state: AdsrState, config: u32) -> AdsrConfig {
        let mode = match state {
            AdsrState::Disabled => AdsrMode::Linear,
            AdsrState::Attack => AdsrConfig::attack_mode(config),
            AdsrState::Decay => AdsrMode::Exponential,
            AdsrState::Sustain => AdsrConfig::sustain_mode(config),
            AdsrState::Release => AdsrConfig::release_mode(config),
        };

        let direction = match state {
            AdsrState::Disabled => AdsrDirection::Increase,
            AdsrState::Attack => AdsrDirection::Increase,
            AdsrState::Decay => AdsrDirection::Decrease,
            AdsrState::Sustain => AdsrConfig::sustain_direction(config),
            AdsrState::Release => AdsrDirection::Decrease,
        };

        let shift = match state {
            AdsrState::Disabled => 0,
            AdsrState::Attack => AdsrConfig::attack_shift(config),
            AdsrState::Decay => AdsrConfig::decay_shift(config),
            AdsrState::Sustain => AdsrConfig::sustain_shift(config),
            AdsrState::Release => AdsrConfig::release_shift(config),
        };

        let step = match state {
            AdsrState::Disabled => 0,
            AdsrState::Attack => AdsrConfig::attack_step(config),
            AdsrState::Decay => -8,
            AdsrState::Sustain => AdsrConfig::sustain_step(config),
            AdsrState::Release => -8,
        };

        let target = match state {
            AdsrState::Disabled => -1,
            AdsrState::Attack => 0x7fff,
            AdsrState::Decay => AdsrConfig::sustain_level(config),
            AdsrState::Sustain => -1,
            AdsrState::Release => 0,
        };

        let next = match state {
            AdsrState::Disabled => AdsrState::Disabled,
            AdsrState::Attack => AdsrState::Decay,
            AdsrState::Decay => AdsrState::Sustain,
            AdsrState::Sustain => AdsrState::Sustain,
            AdsrState::Release => AdsrState::Disabled,
        };

        AdsrConfig {
            mode: mode,
            direction: direction,
            shift: shift,
            step: step,
            target: target,
            next: next,
        }
    }
}

#[derive(Clone, Copy, Default, Deserialize, Serialize)]
pub struct Adsr {
    pub cycles: isize,

    pub state: AdsrState,
    pub config: u32,
    pub volume: i16,
}

impl Adsr {
    pub fn update(&mut self) {
        if self.cycles > 0 {
            self.cycles -= 1;
        }

        let c = AdsrConfig::from(self.state, self.config);

        let mut cycles = 1 << cmp::max(0, c.shift - 11);
        let mut step = c.step << cmp::max(0, 11 - c.shift);

        if c.mode == AdsrMode::Exponential {
            if c.direction == AdsrDirection::Increase {
                if self.volume > 0x6000 {
                    cycles *= 4;
                }
            } else {
                step = (step * self.volume as isize) >> 15;
            }
        }

        if self.cycles <= 0 {
            self.cycles += cycles;
            self.volume = clip(self.volume as isize + step, 0, 0x7fff) as i16;

            if c.target < 0 {
                return;
            }

            if (c.direction == AdsrDirection::Increase && self.volume as isize >= c.target)
                || (c.direction == AdsrDirection::Decrease && self.volume as isize <= c.target)
            {
                self.state = c.next;
                self.cycles = 0;
            }
        }
    }
}
