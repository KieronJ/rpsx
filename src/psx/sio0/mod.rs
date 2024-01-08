pub mod controller;
mod memory_card;

use serde::{Deserialize, Serialize};

use self::controller::Controller;
use self::memory_card::MemoryCard;
use super::intc::{Intc, Interrupt};

use crate::queue::Queue;

#[derive(Deserialize, Serialize)]

struct Mode {
    clk_output_polarity: bool,
    parity_type: bool,
    parity_enable: bool,
    baud_reload_factor: usize,
}

impl Mode {
    pub fn new() -> Mode {
        Mode {
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

#[derive(Deserialize, Serialize)]

struct Control {
    slot: bool,
    ack_interrupt_enable: bool,
    rx_interrupt_enable: bool,
    tx_interrupt_enable: bool,
    rx_interrupt_count: usize,
    rx_enable: bool,
    joy_n_output: bool,
    tx_enable: bool,
}

impl Control {
    pub fn new() -> Control {
        Control {
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
            _ => unreachable!(),
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

#[derive(Deserialize, PartialEq, Serialize)]
enum Device {
    None,
    Controller,
    MemoryCard,
}

#[derive(Deserialize, Serialize)]
pub struct Sio0 {
    controller: Controller,
    mem_card1: MemoryCard,

    active_device: Device,

    baudrate: usize,
    ticks_left: isize,

    in_transfer: bool,
    in_acknowledge: bool,

    interrupt_request: bool,
    ack_input_level: bool,
    rx_parity_error: bool,
    tx_ready_2: bool,
    tx_ready_1: bool,

    mode: Mode,
    control: Control,

    rx_fifo: Queue<u8>,
    tx_fifo: Queue<u8>,
}

impl Sio0 {
    pub fn new() -> Sio0 {
        Sio0 {
            controller: Controller::new(),
            mem_card1: MemoryCard::new("./cards/card1.mcd"),

            active_device: Device::None,

            baudrate: 0,
            ticks_left: 0,

            in_transfer: false,
            in_acknowledge: false,

            interrupt_request: false,
            ack_input_level: false,
            rx_parity_error: false,
            tx_ready_2: false,
            tx_ready_1: false,

            mode: Mode::new(),
            control: Control::new(),

            rx_fifo: Queue::<u8>::new(8),
            tx_fifo: Queue::<u8>::new(1),
        }
    }

    pub fn reset(&mut self) {
        self.mem_card1.reset();
    }

    pub fn reset_device_states(&mut self) {
        // println!("[SIO] resetting device states");
        self.active_device = Device::None;
        self.in_transfer = false;
        self.in_acknowledge = false;
        self.controller.reset_device_state();
        self.mem_card1.reset_device_state();
    }

    pub fn load_memcards(&mut self) {
        self.mem_card1.load("./cards/card1.mcd");
    }

    pub fn tick(&mut self, intc: &mut Intc, clocks: usize) {
        if self.in_transfer {
            self.ticks_left -= clocks as isize;

            if self.ticks_left > 0 {
                return;
            }

            self.in_transfer = false;

            let command = self.tx_fifo.pop();

            if self.control.slot {
                self.rx_fifo.push(0xff);
                return;
            }

            if self.active_device == Device::None {
                if command == 0x01 {
                    self.active_device = Device::Controller;
                } else if command == 0x81 {
                    self.active_device = Device::MemoryCard;
                }
            }

            let mut response = 0xff;
            let mut ack = false;
            let mut enable = false;

            if self.active_device == Device::Controller {
                response = self.controller.response(command);
                ack = self.controller.ack();
                enable = self.controller.enable();

                if ack {
                    self.ticks_left += 338;
                    self.in_acknowledge = true;
                }
            } else if self.active_device == Device::MemoryCard {
                response = self.mem_card1.response(command);
                ack = self.mem_card1.ack();
                enable = self.mem_card1.enable();

                if ack {
                    self.ticks_left += 338;
                    self.in_acknowledge = true;
                }
            }

            if !enable {
                self.active_device = Device::None;
            }

            self.rx_fifo.push(response);

            self.ack_input_level = ack;
            self.tx_ready_2 = true;
        } else if self.in_acknowledge {
            self.ticks_left -= clocks as isize;

            if self.ticks_left < 0 {
                self.in_acknowledge = false;
                self.ack_input_level = false;
                intc.assert_irq(Interrupt::Controller);
            }
        }
    }

    pub fn controller(&mut self) -> &mut Controller {
        &mut self.controller
    }

    pub fn sync(&mut self) {
        self.mem_card1.sync();
    }

    pub fn rx_data(&mut self) -> u32 {
        let data = self.rx_fifo.pop();
        // println!("[SIO0] rx <- {:02X}", data);

        data as u32
    }

    pub fn tx_data(&mut self, value: u32) {
        self.tx_fifo.push(value as u8);
        // println!("[SIO0] tx -> {:02X}", value as u8);

        self.tx_ready_1 = true;
        self.tx_ready_2 = false;

        assert!(!self.in_transfer);
        assert!(!self.in_acknowledge);

        self.ticks_left = (self.baudrate as isize & !1) * 8;
        self.in_transfer = true;
    }

    pub fn status(&mut self) -> u32 {
        let mut value = 0;

        value |= (self.baudrate as u32) << 11;
        value |= (self.interrupt_request as u32) << 9;
        value |= (self.ack_input_level as u32) << 7;
        value |= (self.rx_parity_error as u32) << 3;
        value |= (self.tx_ready_2 as u32) << 2;
        value |= (self.rx_fifo.has_data() as u32) << 1;
        value |= self.tx_ready_1 as u32;

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

        if !self.control.joy_n_output {
            self.reset_device_states();
        }

        if (value & 0x40) != 0 {
            self.write_mode(0);
            self.write_control(0);
            self.write_baud(0);

            self.rx_fifo.clear();
            self.tx_fifo.clear();

            self.tx_ready_1 = true;
            self.tx_ready_2 = true;
        }

        if ((value & 0x10) != 0) && !self.ack_input_level {
            self.interrupt_request = false;
            self.rx_parity_error = false;
        }
    }

    pub fn write_baud(&mut self, value: u16) {
        self.baudrate = value as usize;
    }

    pub fn read_baud(&self) -> u32 {
        self.baudrate as u32
    }
}
