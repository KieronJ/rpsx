use super::helpers;

#[derive(PartialEq, Eq)]
pub struct Timecode {
    minute: usize,
    second: usize,
    sector: usize,
}

impl Timecode {
    pub fn from_bcd(minute: u8, second: u8, sector: u8) -> Self {
        Self {
            minute: helpers::bcd_to_u8(minute) as usize,
            second: helpers::bcd_to_u8(second) as usize,
            sector: helpers::bcd_to_u8(sector) as usize,
        }
    }

    pub fn to_bcd(&self) -> (u8, u8, u8) {
        let minute = helpers::bcd_to_u8(self.minute as u8);
        let second = helpers::bcd_to_u8(self.second as u8);
        let sector = helpers::bcd_to_u8(self.sector as u8);

        (minute, second, sector)
    }

    pub fn to_lba(&self) -> usize {
        self.minute * 60 * 75 +
        self.second * 75 +
        self.sector
    }

    pub fn advance(&mut self) {
        self.sector += 1;

        if self.sector == 75 {
            self.sector = 0;
            self.second += 1;

            if self.second == 60 {
                self.second = 0;
                self.minute += 1;
            }
        }
    }
}