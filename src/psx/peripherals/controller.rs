pub struct Controller {
    state: usize,
    pub digital_mode: bool,

    pub button_select: bool,
    pub button_l3: bool,
    pub button_r3: bool,
    pub button_start: bool,
    pub button_dpad_up: bool,
    pub button_dpad_right: bool,
    pub button_dpad_down: bool,
    pub button_dpad_left: bool,
    pub button_l2: bool,
    pub button_r2: bool,
    pub button_l1: bool,
    pub button_r1: bool,
    pub button_triangle: bool,
    pub button_circle: bool,
    pub button_cross: bool,
    pub button_square: bool,

    pub axis_lx: u8,
    pub axis_ly: u8,
    pub axis_rx: u8,
    pub axis_ry: u8,
}

impl Controller {
    pub fn new() -> Controller {
        Controller {
            state: 0,
            digital_mode: false,

            button_select: false,
            button_l3: false,
            button_r3: false,
            button_start: false,
            button_dpad_up: false,
            button_dpad_right: false,
            button_dpad_down: false,
            button_dpad_left: false,
            button_l2: false,
            button_r2: false,
            button_l1: false,
            button_r1: false,
            button_triangle: false,
            button_circle: false,
            button_cross: false,
            button_square: false,

            axis_lx: 128,
            axis_ly: 128,
            axis_rx: 128,
            axis_ry: 128,
        }
    }

    pub fn response(&mut self, command: u8) -> u8 {
        let mut reply = 0xff;

        match self.state {
            0 => self.state = 1,
            1 => {
                if command == 0x42 {
                    self.state = 2;
                    reply = if self.digital_mode { 0x41 } else { 0x73 };
                } else {
                    self.state = 0;
                }
            },
            2 => {
                reply = 0x5a;
                self.state = 3;
            },
            3 => {
                reply = self.get_switch_state_lo();
                self.state = 4;
            },
            4 => {
                reply = self.get_switch_state_hi();
                self.state = if self.digital_mode { 0 } else { 5 };
            },
            5 => {
                reply = self.axis_rx;
                self.state = 6;
            },
            6 => {
                reply = self.axis_ry;
                self.state = 7;
            },
            7 => {
                reply = self.axis_lx;
                self.state = 8;
            },
            8 => {
                reply = self.axis_ly;
                self.state = 0;
            },
            _ => panic!("[CONTROLLER] [ERROR] Unknown state: {}", self.state),
        };

        reply
    }

    pub fn ack(&self) -> bool {
        self.state != 0
    }

    pub fn enable(&self) -> bool {
        self.state != 0
    }

    fn get_switch_state_hi(&self) -> u8 {
        let mut value = 0;

        value |= (self.button_square as u8) << 7;
        value |= (self.button_cross as u8) << 6;
        value |= (self.button_circle as u8) << 5;
        value |= (self.button_triangle as u8) << 4;
        value |= (self.button_r1 as u8) << 3;
        value |= (self.button_l1 as u8) << 2;
        value |= (self.button_r2 as u8) << 1;
        value |= (self.button_l2 as u8) << 0;

        !value
    }

    fn get_switch_state_lo(&self) -> u8 {
        let mut value = 0;

        value |= (self.button_dpad_left as u8) << 7;
        value |= (self.button_dpad_down as u8) << 6;
        value |= (self.button_dpad_right as u8) << 5;
        value |= (self.button_dpad_up as u8) << 4;
        value |= (self.button_start as u8) << 3;
        value |= (self.button_r3 as u8) << 2;
        value |= (self.button_l3 as u8) << 1;
        value |= self.button_select as u8;

        !value
    }
}
