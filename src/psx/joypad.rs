use std::cell::RefCell;
use std::cmp;
use std::rc::Rc;

use super::controller::Controller;

use queue::Queue;

struct JoypadMode {
    clk_output_polarity: bool,
    parity_type: bool,
    parity_enable: bool,
    baud_reload_factor: usize,
}

impl JoypadMode {
    pub fn new() -> JoypadMode {
        JoypadMode {
            clk_output_polarity: false,
            parity_type: false,
            parity_enable: false,
            baud_reload_factor: 1,
        }
    }

    pub fn write(&mut self, value: u16) {
        self.clk_output_polarity = (value & 0x100) != 0;
        self.parity_type = (value & 0x20) != 0;
        self.parity_enable = (value & 0x10) != 0;
        self.baud_reload_factor = match value & 0x3 {
            0 => 1,
            1 => 1,
            2 => 16,
            3 => 64,
            _ => unreachable!(),
        };
    }
}

struct JoypadControl {
    slot: bool,
    ack_interrupt_enable: bool,
    rx_interrupt_enable: bool,
    tx_interrupt_enable: bool,
    rx_interrupt_count: usize,
    rx_enable: bool,
    joy_n_output: bool,
    tx_enable: bool,
}

impl JoypadControl {
    pub fn new() -> JoypadControl {
        JoypadControl {
            slot: false,
            ack_interrupt_enable: false,
            rx_interrupt_enable: false,
            tx_interrupt_enable: false,
            rx_interrupt_count: 1,
            rx_enable: false,
            joy_n_output: false,
            tx_enable: false,
        }
    }

    pub fn read(&self) -> u16 {
        let mut value = 0;

        value |= (self.slot as u16) << 13;
        value |= (self.ack_interrupt_enable as u16) << 12;
        value |= (self.rx_interrupt_enable as u16) << 11;
        value |= (self.tx_interrupt_enable as u16) << 10;
        value |= match self.rx_interrupt_count {
            1 => 0,
            2 => 1,
            4 => 2,
            8 => 3,
            _ => unreachable!()
        } << 8;
        value |= (self.rx_enable as u16) << 2;
        value |= (self.joy_n_output as u16) << 1;
        value |= self.tx_enable as u16;

        value
    }

    pub fn write(&mut self, value: u16) {
        self.slot = (value & 0x2000) != 0;
        self.ack_interrupt_enable = (value & 0x1000) != 0;
        self.rx_interrupt_enable = (value & 0x800) != 0;
        self.tx_interrupt_enable = (value & 0x400) != 0;
        self.rx_interrupt_count = 1 << ((value & 0x300) >> 8);
        self.rx_enable = (value & 0x4) != 0;
        self.joy_n_output = (value & 0x2) != 0;
        self.tx_enable = (value & 0x1) != 0;
    }
}

pub struct Joypad {
    controller: Rc<RefCell<Controller>>,

    baudrate_reload: usize,
    baudrate_timer: usize,

    interrupt_timer: usize,

    interrupt_request: bool,
    ack_input_level: bool,
    rx_parity_error: bool,
    tx_ready_2: bool,
    tx_ready_1: bool,

    mode: JoypadMode,
    control: JoypadControl,

    rx_fifo: Queue<u8>,
    tx_fifo: Queue<u8>
}

impl Joypad {
    pub fn new(controller: Rc<RefCell<Controller>>) -> Joypad {
        Joypad {
            controller: controller,

            baudrate_reload: 0,
            baudrate_timer: 0,

            interrupt_timer: 0,

            interrupt_request: false,
            ack_input_level: false,
            rx_parity_error: false,
            tx_ready_2: false,
            tx_ready_1: false,

            mode: JoypadMode::new(),
            control: JoypadControl::new(),

            rx_fifo: Queue::<u8>::new(8),
            tx_fifo: Queue::<u8>::new(1),
        }
    }

    pub fn tick(&mut self, clocks: usize) -> bool {
        if self.baudrate_timer > 0 {
            if self.baudrate_timer < clocks {
                self.baudrate_timer = clocks;
            }

            self.baudrate_timer -= clocks;

            if self.baudrate_timer == 0 {
                self.reload_timer();
            }
        }

        if self.tx_fifo.has_data() {
            let command = self.tx_fifo.pop();

            if self.control.slot {
                self.rx_fifo.push(0xff);
                return false;
            }

            let mut controller = self.controller.borrow_mut();

            let response = controller.response(command);
            let ack = controller.ack();

            if ack {
                self.interrupt_timer = 500;
            }

            self.rx_fifo.push(response);

            self.ack_input_level = true;
            self.tx_ready_2 = true;
        }

        if self.interrupt_timer > 0 {
            if self.interrupt_timer < clocks {
                self.interrupt_timer = clocks;
            }

            self.interrupt_timer -= clocks;

            if self.interrupt_timer == 0 {
                self.interrupt_request = true;
                return true;
            }
        }

        false
    }

    pub fn rx_data(&mut self) -> u32 {
        self.rx_fifo.pop() as u32
    }

    pub fn tx_data(&mut self, value: u32) {
        self.tx_fifo.push(value as u8);
        self.tx_ready_1 = true;
        self.tx_ready_2 = false;
    }

    pub fn status(&mut self) -> u32 {
        let mut value = 0;

        value |= (self.baudrate_timer as u32) << 11;
        value |= (self.interrupt_request as u32) << 9;
        value |= (self.ack_input_level as u32) << 7;
        value |= (self.rx_parity_error as u32) << 3;
        value |= (self.tx_ready_2 as u32) << 2;
        value |= (self.rx_fifo.has_data() as u32) << 1;
        value |= self.tx_ready_1 as u32;

        self.ack_input_level = false;

        value
    }

    pub fn write_mode(&mut self, value: u16) {
        self.mode.write(value);
    }

    pub fn read_control(&self) -> u32 {
        self.control.read() as u32
    }

    pub fn write_control(&mut self, value: u16) {
        self.control.write(value);

        if (value & 0x40) != 0 {
            self.write_mode(0);
            self.write_control(0);
            self.write_baud(0);

            self.rx_fifo.clear();
            self.tx_fifo.clear();

            self.tx_ready_1 = true;
            self.tx_ready_2 = true;
        }

        if (value & 0x10) != 0 {
            self.interrupt_request = false;
            self.rx_parity_error = false;
        }
    }

    pub fn write_baud(&mut self, value: u16) {
        self.baudrate_reload = value as usize;

        self.reload_timer();
    }

    pub fn read_baud(&self) -> u32 {
        self.baudrate_reload as u32
    }

    fn reload_timer(&mut self) {
        let timer_reload = self.baudrate_reload * self.mode.baud_reload_factor;
        self.baudrate_timer = cmp::max(0x20, timer_reload & !0x1);
    }
}