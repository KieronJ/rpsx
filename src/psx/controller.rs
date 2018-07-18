use sdl2::keyboard::Keycode;

use queue::Queue;

pub struct Controller {
    ack: bool,

    enable: bool,
    in_command: bool,

    buffer: Queue<u8>,

    button_select: bool,
    button_start: bool,
    button_joypad_up: bool,
    button_joypad_right: bool,
    button_joypad_down: bool,
    button_joypad_left: bool,
    button_l2: bool,
    button_r2: bool,
    button_l1: bool,
    button_r1: bool,
    button_triangle: bool,
    button_circle: bool,
    button_cross: bool,
    button_square: bool,
}

impl Controller {
    pub fn new() -> Controller {
        Controller {
            ack: false,

            enable: false,
            in_command: false,

            buffer: Queue::<u8>::new(8),

            button_select: true,
            button_start: true,
            button_joypad_up: true,
            button_joypad_right: true,
            button_joypad_down: true,
            button_joypad_left: true,
            button_l2: true,
            button_r2: true,
            button_l1: true,
            button_r1: true,
            button_triangle: true,
            button_circle: true,
            button_cross: true,
            button_square: true,
        }
    }

    fn get_switch_state(&self) -> u16 {
        let mut value = 0;

        value |= (self.button_square as u16) << 15;
        value |= (self.button_cross as u16) << 14;
        value |= (self.button_circle as u16) << 13;
        value |= (self.button_triangle as u16) << 12;
        value |= (self.button_r1 as u16) << 11;
        value |= (self.button_l1 as u16) << 10;
        value |= (self.button_r2 as u16) << 9;
        value |= (self.button_l2 as u16) << 8;
        value |= (self.button_joypad_left as u16) << 7;
        value |= (self.button_joypad_down as u16) << 6;
        value |= (self.button_joypad_right as u16) << 5;
        value |= (self.button_joypad_up as u16) << 4;
        value |= (self.button_start as u16) << 3;
        value |= 1 << 2;
        value |= 1 << 1;
        value |= self.button_select as u16;

        value
    }

    pub fn response(&mut self, command: u8) -> u8 {
        if !self.enable {
            if command == 0x01 {
                self.enable = true;
                self.ack = true;
            }

            return 0xff;
        }

        if !self.in_command {
            if command == 0x42 {
                self.in_command = true;
                self.begin_read();
            } else {
                self.ack = false;
                self.enable = false;

                return 0xff;
            }
        }

        let response = self.buffer.pop();

        if self.buffer.empty() {
            self.enable = false;
            self.in_command = false;

            self.ack = false;
        }

        response
    }

    pub fn ack(&self) -> bool {
        self.ack
    }

    pub fn set(&mut self, keycode: Keycode, state: bool) {
        match keycode {
            Keycode::W => self.button_joypad_up = state,
            Keycode::A => self.button_joypad_left = state,
            Keycode::S => self.button_joypad_down = state,
            Keycode::D => self.button_joypad_right = state,
            Keycode::Kp8 => self.button_triangle = state,
            Keycode::Kp4 => self.button_square = state,
            Keycode::Kp2 => self.button_cross = state,
            Keycode::Kp6 => self.button_circle = state,
            Keycode::Q => self.button_select = state,
            Keycode::E => self.button_start = state,
            Keycode::Num1 => self.button_l1 = state,
            Keycode::Num2 => self.button_l2 = state,
            Keycode::Num3 => self.button_r1 = state,
            Keycode::Num4 => self.button_r2 = state,
            _ => (),
        };
    }

    fn begin_read(&mut self) {
        let state = self.get_switch_state();

        self.buffer.push(0x41);
        self.buffer.push(0x5a);
        self.buffer.push(state as u8);
        self.buffer.push((state >> 8) as u8);
    }
}