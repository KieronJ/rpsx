use std::str;

use serde::{Deserialize, Serialize};

const DUART_SRA: u32 = 0x1f802021;
const DUART_THRA: u32 = 0x1f802023;

const DUART_SR_TXRDY: u8 = 0x4;

#[derive(Deserialize, Serialize)]
pub struct Exp2 {
    tx_buf: Vec<u8>,
}

impl Exp2 {
    pub fn new() -> Exp2 {
        Exp2 {
            tx_buf: Vec::new(),
        }
    }

    fn tx_byte(&mut self, byte: u8) {
        if byte == 0xd {
            return;
        }

        if byte == 0xa {
            if self.tx_buf.len() != 0 {
                println!("{}", str::from_utf8(&self.tx_buf).unwrap());
                self.tx_buf.clear();
            }

            return;
        }

        self.tx_buf.push(byte);
    }

    pub fn read8(&mut self, address: u32) -> u8 {
        if address == DUART_SRA {
            return DUART_SR_TXRDY;
        }

        0
    }

    pub fn write8(&mut self, address: u32, value: u8) {
        if address == DUART_THRA {
            self.tx_byte(value);
        }
    }
}