mod container;
mod headers;
mod helpers;
mod timecode;

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use serde::{Deserialize, Serialize};

use timecode::Timecode;

use crate::psx::adpcm::{ADPCM_FILTERS, ADPCM_ZIGZAG_TABLE};
use crate::queue::Queue;
use crate::util::{bcd_to_u8, clip, u8_to_bcd};

use super::intc::{Intc, Interrupt};
use super::spu::Spu;

pub const SECTORS_PER_SECOND: u64 = 75;
pub const SECTORS_PER_MINUTE: u64 = 60 * SECTORS_PER_SECOND;
pub const BYTES_PER_SECTOR: u64 = 2352;
pub const LEAD_IN_SECTORS: u64 = 2 * SECTORS_PER_SECOND;

pub const ADDRESS_OFFSET: usize = 12;
pub const DATA_OFFSET: usize = 24;

#[derive(Clone, Copy, PartialEq)]
enum CdromSubheaderMode {
    Video,
    Audio,
    Data,
    Invalid,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
struct CdromSubheader {
    file: u8,
    channel: u8,
    submode: u8,
    codinginfo: u8,
}

impl CdromSubheader {
    pub fn new() -> Self {
        Self {
            file: 0,
            channel: 0,
            submode: 0,
            codinginfo: 0,
        }
    }

    pub fn from_slice(bytes: &[u8]) -> Self {
        Self {
            file: bytes[0],
            channel: bytes[1] & 0x1f,
            submode: bytes[2],
            codinginfo: bytes[3],
        }
    }

    pub fn mode(self) -> CdromSubheaderMode {
        use self::CdromSubheaderMode::*;

        match self.submode & 0xe {
            0x2 => Video,
            0x4 => Audio,
            0x0 | 0x8 => Data,
            _ => Invalid,
        }
    }

    pub fn realtime(self) -> bool {
        (self.submode & 0x40) != 0
    }

    pub fn channels(self) -> usize {
        match self.codinginfo & 0x3 {
            0 => 1,
            1 => 2,
            _ => panic!("[CDROM] [ERROR] Reserved ADPCM format"),
        }
    }

    pub fn sampling_rate(self) -> usize {
        match self.codinginfo & 0xc {
            0x0 => 37800,
            0x4 => 18900,
            _ => 0,
        }
    }

    pub fn bit_depth(self) -> usize {
        match self.codinginfo & 0x30 {
            0x0 => 4,
            0x10 => 8,
            _ => 0,
        }
    }
}

#[derive(Clone, Copy, Deserialize, Serialize)]
struct CdromHeader {
    minute: u8,
    second: u8,
    sector: u8,
    mode: u8,
}

impl CdromHeader {
    pub fn new() -> Self {
        Self {
            minute: 0,
            second: 0,
            sector: 0,
            mode: 0,
        }
    }

    pub fn from_slice(bytes: &[u8]) -> Self {
        Self {
            minute: bcd_to_u8(bytes[0]),
            second: bcd_to_u8(bytes[1]),
            sector: bcd_to_u8(bytes[2]),
            mode: bytes[3],
        }
    }
}

#[derive(Deserialize, Serialize)]
struct CdromSubchannelQ {
    pub track: u8,
    pub index: u8,
    pub mm: u8,
    pub ss: u8,
    pub ff: u8,
    pub amm: u8,
    pub ass: u8,
    pub aff: u8,
}

impl CdromSubchannelQ {
    pub fn new() -> Self {
        Self {
            track: 0,
            index: 0,
            mm: 0,
            ss: 0,
            ff: 0,
            amm: 0,
            ass: 0,
            aff: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
enum CdromIndex {
    Index0,
    Index1,
    Index2,
    Index3,
}

#[derive(Clone, Copy, Deserialize, PartialEq, Serialize)]
enum CdromControllerMode {
    Idle,
    ParameterTransfer,
    CommandTransfer,
    CommandExecute,
    ResponseClear,
    ResponseTransfer,
    InterruptTransfer,
}

#[derive(Clone, Copy, Deserialize, PartialEq, Serialize)]
enum CdromDriveMode {
    Idle,
    GetStat,
    Seek,
    Read,
    Play,
}

#[derive(Clone, Copy, Deserialize, PartialEq, Serialize)]
enum CdromSecondResponseMode {
    Idle,
    GetID,
    GetStat,
}

#[derive(PartialEq)]
enum CdromSectorMode {
    Adpcm,
    Data,
    Ignore,
}

static COMMAND_NAMES: [&'static str; 32] = [
    "CdlSync",
    "CdlNop",
    "CdlSetloc",
    "CdlPlay",
    "CdlForward",
    "CdlBackward",
    "CdlReadN",
    "CdlStandby",
    "CdlStop",
    "CdlPause",
    "CdlReset",
    "CdlMute",
    "CdlDemute",
    "CdlSetfilter",
    "CdlSetmode",
    "CdlGetparam",
    "CdlGetlocL",
    "CdlGetlocP",
    "? 0x12",
    "CdlGetTN",
    "CdlGetTD",
    "CdlSeekL",
    "CdlSeekP",
    "? 0x17",
    "? 0x18",
    "CdlTest",
    "CdlGetID",
    "CdlReadS",
    "? 0x1c",
    "? 0x1d",
    "CdlGetTOC",
    "? 0x1f"
];

#[derive(Deserialize, Serialize)]
pub struct Cdrom {
    index: CdromIndex,

    interrupt_enable: u8,
    interrupt_flags: u8,

    command: Option<u8>,

    playing: bool,
    seeking: bool,
    reading: bool,

    parameter_buffer: Queue<u8>,
    response_buffer: Queue<u8>,
    data_buffer: Vec<u8>,
    data_buffer_ptr: usize,

    want_data: bool,

    data_busy: bool,

    seek_unprocessed: bool,
    seek_minute: u8,
    seek_second: u8,
    seek_sector: u8,

    filter_file: u8,
    filter_channel: u8,

    sector_header: CdromHeader,
    sector_subheader: CdromSubheader,
    sector: Vec<u8>,

    adpcm_buffers: [Vec<i16>; 2],
    adpcm_prev_samples: [[i16; 2]; 2],

    mode_double_speed: bool,
    mode_adpcm: bool,
    mode_sector_size: bool,
    mode_filter: bool,
    mode_report: bool,

    controller_counter: isize,
    controller_mode: CdromControllerMode,

    controller_interrupt_flags: u8,

    controller_command: u8,

    controller_parameter_buffer: Queue<u8>,
    controller_response_buffer: Queue<u8>,

    drive_counter: isize,
    drive_mode: CdromDriveMode,
    next_drive_mode: CdromDriveMode,

    drive_interrupt_pending: bool,
    drive_pending_stat: u8,

    second_response_counter: isize,
    second_response_mode: CdromSecondResponseMode,

    drive_seek_minute: u8,
    drive_seek_second: u8,
    drive_seek_sector: u8,

    ldrive_seek_sector: u8,
    ldrive_seek_second: u8,
    ldrive_seek_minute: u8,

    last_subq: CdromSubchannelQ,

    #[serde(skip)]
    game_file: Option<File>,

    sixstep: usize,
    ringbuf: [[i16; 0x20]; 2],
}

impl Cdrom {
    pub fn new(game_filepath: &str) -> Cdrom {
        let path = Path::new(game_filepath);

        if !path.is_file() {
            panic!("ERROR: file does not exist: {}", path.display())
        }

        Cdrom {
            index: CdromIndex::Index0,

            interrupt_enable: 0,
            interrupt_flags: 0,

            command: None,

            playing: false,
            seeking: false,
            reading: false,

            parameter_buffer: Queue::<u8>::new(16),
            response_buffer: Queue::<u8>::new(16),
            data_buffer: vec![0; 0x930],
            data_buffer_ptr: 0,

            want_data: false,

            data_busy: false,

            seek_unprocessed: false,
            seek_minute: 0,
            seek_second: 0,
            seek_sector: 0,

            filter_file: 0,
            filter_channel: 0,

            sector_header: CdromHeader::new(),
            sector_subheader: CdromSubheader::new(),
            sector: vec![0u8; 0x930],

            adpcm_buffers: [Vec::new(), Vec::new()],
            adpcm_prev_samples: [[0; 2]; 2],

            mode_double_speed: false,
            mode_adpcm: false,
            mode_sector_size: false,
            mode_filter: false,
            mode_report: false,

            controller_counter: 0,
            controller_mode: CdromControllerMode::Idle,

            controller_interrupt_flags: 0,

            controller_command: 0,

            controller_parameter_buffer: Queue::<u8>::new(16),
            controller_response_buffer: Queue::<u8>::new(16),

            drive_counter: 0,
            drive_mode: CdromDriveMode::Idle,
            next_drive_mode: CdromDriveMode::Idle,

            drive_interrupt_pending: false,
            drive_pending_stat: 0,

            second_response_counter: 0,
            second_response_mode: CdromSecondResponseMode::Idle,

            drive_seek_minute: 0,
            drive_seek_second: 0,
            drive_seek_sector: 0,

            ldrive_seek_sector: 0,
            ldrive_seek_second: 0,
            ldrive_seek_minute: 0,

            last_subq: CdromSubchannelQ::new(),

            game_file: Some(File::open(path).unwrap()),

            sixstep: 0,
            ringbuf: [[0; 0x20]; 2],
        }
    }

    pub fn reset(&mut self) {
    }

    pub fn load_disc(&mut self, filepath: &str) {
        let path = Path::new(filepath);

        if !path.is_file() {
            panic!("ERROR: file does not exist: {}", path.display())
        }

        self.game_file = Some(File::open(path).unwrap());
    }

    pub fn tick(&mut self, intc: &mut Intc, spu: &mut Spu, clocks: usize) {
        self.tick_second_response(clocks);
        self.tick_drive(spu, clocks);
        self.tick_controller(clocks);

        if (self.interrupt_enable & self.interrupt_flags & 0x1f) != 0 {
            intc.assert_irq(Interrupt::Cdrom);
        }
    }

    fn tick_controller(&mut self, clocks: usize) {
        self.controller_counter -= clocks as isize;

        if self.controller_counter > 0 {
            return;
        }

        match self.controller_mode {
            CdromControllerMode::Idle => {
                if let Some(_) = self.command {
                    if self.parameter_buffer.has_data() {
                        self.controller_mode = CdromControllerMode::ParameterTransfer;
                    } else {
                        self.controller_mode = CdromControllerMode::CommandTransfer;
                    }
                }

                self.controller_counter += clocks as isize;
            }
            CdromControllerMode::ParameterTransfer => {
                if self.parameter_buffer.has_data() {
                    let parameter = self.parameter_buffer.pop();
                    self.controller_parameter_buffer.push(parameter);
                } else {
                    self.controller_mode = CdromControllerMode::CommandTransfer;
                }

                self.controller_counter += 10;
            }
            CdromControllerMode::CommandTransfer => {
                self.controller_command = self.command.unwrap();
                self.command = None;

                self.controller_mode = CdromControllerMode::CommandExecute;
                self.controller_counter += 10;
            }
            CdromControllerMode::CommandExecute => {
                let command = self.controller_command;

                self.controller_counter += 10;

                self.controller_response_buffer.clear();

                self.execute_command(command);
                self.controller_parameter_buffer.clear();

                self.controller_mode = CdromControllerMode::ResponseClear;
            }
            CdromControllerMode::ResponseClear => {
                if self.response_buffer.has_data() {
                    self.response_buffer.pop();
                } else {
                    self.controller_mode = CdromControllerMode::ResponseTransfer;
                }

                self.controller_counter += 10;
            }
            CdromControllerMode::ResponseTransfer => {
                if self.controller_response_buffer.has_data() {
                    let response = self.controller_response_buffer.pop();
                    self.response_buffer.push(response);
                } else {
                    self.controller_mode = CdromControllerMode::InterruptTransfer;
                }

                self.controller_counter += 10;
            }
            CdromControllerMode::InterruptTransfer => {
                if self.interrupt_flags == 0 {
                    self.interrupt_flags = self.controller_interrupt_flags;

                    self.controller_mode = CdromControllerMode::Idle;
                    self.controller_counter += 10;
                } else {
                    self.controller_counter += 1;
                }
            }
        }
    }

    fn tick_second_response(&mut self, clocks: usize) {
        self.second_response_counter -= clocks as isize;

        if self.second_response_counter > 0 {
            return;
        }

        match self.second_response_mode {
            CdromSecondResponseMode::Idle => self.second_response_counter += clocks as isize,
            CdromSecondResponseMode::GetID => {
                if self.interrupt_flags == 0 {
                    self.controller_response_buffer.push(0x02);
                    self.controller_response_buffer.push(0x00);

                    self.controller_response_buffer.push(0x20);
                    self.controller_response_buffer.push(0x00);

                    self.controller_response_buffer.push('S' as u8);
                    self.controller_response_buffer.push('C' as u8);
                    self.controller_response_buffer.push('E' as u8);
                    self.controller_response_buffer.push('A' as u8);

                    self.controller_interrupt_flags = 0x2;

                    self.controller_mode = CdromControllerMode::ResponseClear;
                    self.controller_counter += 10;

                    self.second_response_mode = CdromSecondResponseMode::Idle;
                }

                self.second_response_counter += 1;
            }
            CdromSecondResponseMode::GetStat => {
                if self.interrupt_flags == 0 {
                    self.push_stat();

                    self.controller_interrupt_flags = 0x2;

                    self.controller_mode = CdromControllerMode::ResponseClear;
                    self.controller_counter += 10;

                    self.second_response_mode = CdromSecondResponseMode::Idle;
                }

                self.second_response_counter += 1;
            }
        }
    }

    fn tick_drive(&mut self, spu: &mut Spu, clocks: usize) {
        self.drive_counter -= clocks as isize;

        if self.drive_counter > 0 {
            return;
        }

        match self.drive_mode {
            CdromDriveMode::Idle => self.drive_counter += clocks as isize,
            CdromDriveMode::GetStat => {
                if self.interrupt_flags == 0 {
                    self.push_stat();

                    self.controller_interrupt_flags = 0x2;

                    self.controller_mode = CdromControllerMode::ResponseClear;
                    self.controller_counter += 10;

                    self.drive_mode = CdromDriveMode::Idle;
                }

                self.drive_counter += 1;
            }
            CdromDriveMode::Seek => {
                self.seek_unprocessed = false;

                self.drive_seek_minute = self.seek_minute;
                self.drive_seek_second = self.seek_second;
                self.drive_seek_sector = self.seek_sector;

                self.last_subq.track = 1;
                self.last_subq.index = 1;
                self.last_subq.mm = self.seek_minute;
                self.last_subq.ss = self.seek_second - 2;
                self.last_subq.ff = self.seek_sector;
                self.last_subq.amm = self.seek_minute;
                self.last_subq.ass = self.seek_second;
                self.last_subq.aff = self.seek_sector;

                self.reading = false;
                self.seeking = false;
                self.playing = false;

                if self.next_drive_mode == CdromDriveMode::Read {
                    self.reading = true;
                    self.playing = false;

                    self.drive_counter += 44100
                        / match self.mode_double_speed {
                            true => 150,
                            false => 75,
                        };
                } else if self.next_drive_mode == CdromDriveMode::Play {
                    self.reading = false;
                    self.playing = true;

                    self.drive_counter += 44100
                        / match self.mode_double_speed {
                            true => 150,
                            false => 75,
                        };
                } else {
                    self.drive_counter += 10;
                }

                self.drive_mode = self.next_drive_mode;
            }
            CdromDriveMode::Play => {
                if !self.playing {
                    self.drive_mode = CdromDriveMode::Idle;
                    self.drive_counter += 1;
                    return;
                }

                let cursor = self.get_seek_location();
                let mut data = [0u8; 0x930];

                if let Some(file) = self.game_file.as_mut() {
                    file.seek(SeekFrom::Start(cursor)).unwrap();
                    file.read_exact(&mut data);
                } else {
                    panic!("no game file");
                }

                for i in 0..0x24c {
                    let left = (data[i * 4] as u16) | ((data[i * 4 + 1] as u16) << 8);
                    let right = (data[i * 4 + 2] as u16) | ((data[i * 4 + 3] as u16) << 8);

                    spu.cd_push(left as i16, right as i16);
                }

                if self.mode_report {
                    let track = 1;
                    let index = 1;
                    let amm = u8_to_bcd(self.drive_seek_minute);
                    let ass = u8_to_bcd(self.drive_seek_second);
                    let aff = u8_to_bcd(self.drive_seek_sector);
                    let peaklo = 0;
                    let peakhi = 0;

                    match aff {
                        0x00 | 0x20 | 0x40 | 0x60 => {
                            self.push_stat();
                            self.controller_response_buffer.push(track);
                            self.controller_response_buffer.push(index);
                            self.controller_response_buffer.push(amm);
                            self.controller_response_buffer.push(ass);
                            self.controller_response_buffer.push(aff);
                            self.controller_response_buffer.push(peaklo);
                            self.controller_response_buffer.push(peakhi);

                            self.controller_interrupt_flags = 0x1;

                            self.controller_mode = CdromControllerMode::ResponseClear;
                            self.controller_counter += 10;
                        }
                        0x10 | 0x30 | 0x50 | 0x70 => {
                            self.push_stat();
                            self.controller_response_buffer.push(track);
                            self.controller_response_buffer.push(index);
                            self.controller_response_buffer.push(amm);
                            self.controller_response_buffer.push(ass + 0x80);
                            self.controller_response_buffer.push(aff);
                            self.controller_response_buffer.push(peaklo);
                            self.controller_response_buffer.push(peakhi);

                            self.controller_interrupt_flags = 0x1;

                            self.controller_mode = CdromControllerMode::ResponseClear;
                            self.controller_counter += 10;
                        }
                        _ => ()
                    }
                }

                self.drive_seek_sector += 1;

                if self.drive_seek_sector >= 75 {
                    self.drive_seek_sector = 0;
                    self.drive_seek_second += 1;
                }

                if self.drive_seek_second >= 60 {
                    self.drive_seek_second = 0;
                    self.drive_seek_minute += 1;
                }

                self.drive_counter += 44100
                    / match self.mode_double_speed {
                        true => 150,
                        false => 75,
                    };
            }
            CdromDriveMode::Read => {
                if !self.reading {
                    self.drive_mode = CdromDriveMode::Idle;
                    self.drive_counter += 1;
                    return;
                }

                self.push_stat();

                self.data_busy = true;

                let cursor = self.get_seek_location();
                let mut info = [0u8; 0x18];

                if let Some(file) = self.game_file.as_mut() {
                    file.seek(SeekFrom::Start(cursor)).unwrap();
                    file.read_exact(&mut info).unwrap();
                } else {
                    panic!("no game file");
                }

                let header = CdromHeader::from_slice(&info[0xc..]);
                let subheader = CdromSubheader::from_slice(&info[0x10..]);

                self.sector_header = header;
                self.sector_subheader = subheader;

                self.last_subq.track = 1;
                self.last_subq.index = 1;
                self.last_subq.mm = header.minute;
                self.last_subq.ss = header.second - 2;
                self.last_subq.ff = header.sector;
                self.last_subq.amm = header.minute;
                self.last_subq.ass = header.second;
                self.last_subq.aff = header.sector;

                let mut mode = CdromSectorMode::Adpcm;

                if !self.mode_adpcm || (subheader.mode() != CdromSubheaderMode::Audio) || !subheader.realtime() {
                    mode = CdromSectorMode::Data;
                }

                if mode == CdromSectorMode::Adpcm && self.mode_filter && ((self.filter_file != subheader.file) || (self.filter_channel != subheader.channel)) {
                    mode = CdromSectorMode::Ignore;
                }

                //if mode == CdromSectorMode::Data
                //    && self.mode_filter
                //    && (subheader.mode() == CdromSubheaderMode::Audio)
                //    && subheader.realtime()
                //{
                //    mode = CdromSectorMode::Ignore;
                //}

                let sm = self.drive_seek_minute;
                let ss = self.drive_seek_second;
                let sf = self.drive_seek_sector;

                if header.minute != sm || header.second != ss || header.sector != sf {
                    println!("[CDROM] [ERROR] Sector with mismatched header detected");
                    println!(
                        "Expected {}:{}:{} found {}:{}:{}",
                        sm, ss, sf, header.minute, header.second, header.sector
                    );
                }

                if header.mode != 2 {
                    println!("[CDROM] [ERROR] Unsupported MODE{} sector", header.mode);
                }

                self.ldrive_seek_sector = sm;
                self.ldrive_seek_second = ss;
                self.ldrive_seek_minute = sf;

                self.drive_seek_sector += 1;

                if self.drive_seek_sector >= 75 {
                    self.drive_seek_sector = 0;
                    self.drive_seek_second += 1;
                }

                if self.drive_seek_second >= 60 {
                    self.drive_seek_second = 0;
                    self.drive_seek_minute += 1;
                }

                match mode {
                    CdromSectorMode::Adpcm => {
                        if subheader.bit_depth() != 4 {
                            panic!("[CDROM] [ERROR] Unsupported bit depth");
                        }

                        //DECODE XA-ADPCM
                        let channels = subheader.channels();
                        let sampling_rate = subheader.sampling_rate();

                        let mut data = [0u8; 0x914];

                        if let Some(file) = self.game_file.as_mut() {
                            file.read_exact(&mut data).unwrap();
                        } else {
                            panic!("no game file");
                        }

                        for i in 0..0x12 {
                            self.decode_adpcm_blocks(&data[i * 0x80..], channels);
                        }

                        let times = match sampling_rate {
                            18900 => 2,
                            37800 => 1,
                            _ => unreachable!()
                        };

                        for channel in 0..channels {
                            for _ in 0..times {
                                for i in 0..self.adpcm_buffers[channel].len() {
                                    self.ringbuf[channel][i & 0x1f] = self.adpcm_buffers[channel][i];
                                    self.sixstep += 1;

                                    if self.sixstep == 6 {
                                        self.sixstep = 0;

                                        for j in 0..7 {
                                            let sample = self.zigzag_interpolate(i + 1, self.ringbuf[channel], ADPCM_ZIGZAG_TABLE[j]);

                                            match (channel, channels) {
                                                (_, 1) => spu.cd_push(sample, sample),
                                                (0, 2) => spu.cd_push_left(sample),
                                                (1, 2) => spu.cd_push_right(sample),
                                                _ => unreachable!(),
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        self.adpcm_buffers[0].clear();
                        self.adpcm_buffers[1].clear();
                    }
                    CdromSectorMode::Data => {
                        if let Some(file) = self.game_file.as_mut() {
                            file.seek(SeekFrom::Start(cursor)).unwrap();
                            file.read_exact(&mut self.sector).unwrap();
                        } else {
                            panic!("no game file");
                        }

                        // TODO: stat
                        if self.drive_interrupt_pending {
                            println!("[CDC] [WARN] Got drive interrupt whilst already pending");
                        }

                        if self.interrupt_flags == 0 {
                            self.interrupt_flags = 0x1;
                            self.response_buffer.push(self.get_stat());
                        } else {
                            self.drive_interrupt_pending = true;
                            self.drive_pending_stat = self.get_stat();
                        }

                        //self.controller_mode = CdromControllerMode::ResponseClear;
                        //self.controller_counter += 10;
                    }
                    CdromSectorMode::Ignore => {}, // Sector is skipped by Cdrom controller
                };

                self.drive_counter += 44100 / match self.mode_double_speed {
                    true => 150,
                    false => 75,
                };
            }
        };
    }

    fn zigzag_interpolate(&self, index: usize, buffer: [i16; 0x20], table: [i32; 29]) -> i16 {
        let mut sum = 0;

        for i in 1..30 {
            sum += ((buffer[(index - i) & 0x1f] as i32) * table[i - 1]) / 0x8000;
        }

        clip(sum, -0x8000, 0x7fff) as i16
    }

    fn decode_adpcm_blocks(&mut self, data: &[u8], channels: usize) {
        for i in 0..8 {
            let channel = match channels {
                1 => 0,
                2 => i & 0x1,
                _ => unreachable!(),
            };

            self.decode_adpcm_block(data, channel, i);
        }
    }

    fn decode_adpcm_block(&mut self, src: &[u8], channel: usize, block: usize) {
        let header = src[0x4 + block];

        let filter = ((header & 0x30) >> 4) as usize;
        let mut shift = header & 0xf;

        if shift > 12 {
            shift = 9;
        }

        for i in 0..28 {
            let mut sample = src[0x10 + (i * 4) + (block / 2)] as u16;

            if (block & 0x1) != 0 {
                sample >>= 4;
            }

            sample &= 0xf;

            let mut sample = (sample << 12) as i16 as i32;
            sample >>= shift;

            let mut quant = 32;
            quant += self.adpcm_prev_samples[channel][0] as i32 * ADPCM_FILTERS[filter][0] as i32;
            quant -= self.adpcm_prev_samples[channel][1] as i32 * ADPCM_FILTERS[filter][1] as i32;

            sample = clip(sample + (quant / 64), -0x8000, 0x7fff);

            self.adpcm_buffers[channel].push(sample as i16);
            self.adpcm_prev_samples[channel][1] = self.adpcm_prev_samples[channel][0];
            self.adpcm_prev_samples[channel][0] = sample as i16;
        }
    }

    fn execute_command(&mut self, command: u8) {
        if command >= 0x20 {
            panic!("[CDC] Invalid command {}", command);
        }

        //println!("[CDC] {}", COMMAND_NAMES[command as usize]);

        let mut interrupt = 0x3;

        match command {
            0x01 => {
                self.push_stat();
            }
            0x02 => {
                self.push_stat();

                let mm = self.controller_parameter_buffer.pop();
                let ss = self.controller_parameter_buffer.pop();
                let ff = self.controller_parameter_buffer.pop();

                //println!("({:02x}, {:02x}, {:02x})", mm, ss, ff);

                self.seek_unprocessed = true;

                self.seek_minute = bcd_to_u8(mm);
                self.seek_second = bcd_to_u8(ss);
                self.seek_sector = bcd_to_u8(ff);
            }
            0x03 => {
                self.controller_parameter_buffer.pop();

                if self.seek_unprocessed {
                    self.seeking = true;
                    self.reading = false;
                    self.playing = false;

                    self.drive_mode = CdromDriveMode::Seek;
                    self.next_drive_mode = CdromDriveMode::Play;

                    self.drive_counter += match self.mode_double_speed {
                        false => 28,
                        true => 14,
                    };
                } else {
                    self.seeking = false;
                    self.reading = false;
                    self.playing = true;

                    self.drive_mode = CdromDriveMode::Play;

                    self.drive_counter += 44100
                        / match self.mode_double_speed {
                            true => 150,
                            false => 75,
                        };
                }

                self.push_stat();
            }
            0x06 => {
                if self.seek_unprocessed {
                    self.seeking = true;
                    self.reading = false;
                    self.playing = false;

                    self.drive_mode = CdromDriveMode::Seek;
                    self.next_drive_mode = CdromDriveMode::Read;

                    self.drive_counter += match self.mode_double_speed {
                        false => 280,
                        true => 140,
                    };
                } else {
                    self.seeking = false;
                    self.reading = true;
                    self.playing = false;

                    self.drive_mode = CdromDriveMode::Read;

                    self.drive_counter += 44100
                        / match self.mode_double_speed {
                            true => 150,
                            false => 75,
                        };
                }

                self.push_stat();
            }
            0x07 => {
                self.push_stat();
                self.controller_response_buffer.push(0x20);

                interrupt = 0x5;
            }
            0x09 => {
                self.push_stat();

                if self.playing == false && self.reading == false && self.seeking == false {
                    self.second_response_counter += 10;
                } else {
                    self.second_response_counter += match self.mode_double_speed {
                        false => 2800,
                        true => 1400,
                    };
                }

                self.playing = false;
                self.reading = false;
                self.seeking = false;

                //self.drive_seek_sector = self.ldrive_seek_sector;
                //self.drive_seek_second = self.ldrive_seek_second;
                //self.drive_seek_minute = self.ldrive_seek_minute;

                self.second_response_mode = CdromSecondResponseMode::GetStat;
            }
            0x0a => {
                self.push_stat();

                self.mode_double_speed = false;
                self.mode_sector_size = false;
                self.reading = false;
                self.playing = false;
                self.seeking = false;

                self.second_response_mode = CdromSecondResponseMode::GetStat;
                self.second_response_counter += 10;
            }
            0x0b => {
                self.push_stat();
            }
            0x0c => {
                self.push_stat();
            }
            0x0d => {
                let file = self.controller_parameter_buffer.pop();
                let channel = self.controller_parameter_buffer.pop();

                self.filter_file = file;
                self.filter_channel = channel & 0x1f;

                self.push_stat();
            }
            0x0e => {
                self.push_stat();

                let mode = self.controller_parameter_buffer.pop();

                self.mode_double_speed = (mode & 0x80) != 0;
                self.mode_adpcm = (mode & 0x40) != 0;
                self.mode_sector_size = (mode & 0x20) != 0;
                self.mode_filter = (mode & 0x8) != 0;
                self.mode_report = (mode & 0x4) != 0;
            }
            0x10 => {
                let amm = u8_to_bcd(self.sector_header.minute);
                let ass = u8_to_bcd(self.sector_header.second);
                let aff = u8_to_bcd(self.sector_header.sector);
                let mode = self.sector_header.mode;
                let file = self.sector_subheader.file;
                let channel = self.sector_subheader.channel;
                let sm = self.sector_subheader.submode;
                let ci = self.sector_subheader.codinginfo;

                self.controller_response_buffer.push(amm);
                self.controller_response_buffer.push(ass);
                self.controller_response_buffer.push(aff);
                self.controller_response_buffer.push(mode);
                self.controller_response_buffer.push(file);
                self.controller_response_buffer.push(channel);
                self.controller_response_buffer.push(sm);
                self.controller_response_buffer.push(ci);
            }
            0x11 => {
                self.controller_response_buffer.push(self.last_subq.track);
                self.controller_response_buffer.push(self.last_subq.index);
                self.controller_response_buffer.push(u8_to_bcd(self.last_subq.mm));
                self.controller_response_buffer.push(u8_to_bcd(self.last_subq.ss));
                self.controller_response_buffer.push(u8_to_bcd(self.last_subq.ff));
                self.controller_response_buffer.push(u8_to_bcd(self.last_subq.amm));
                self.controller_response_buffer.push(u8_to_bcd(self.last_subq.ass));
                self.controller_response_buffer.push(u8_to_bcd(self.last_subq.aff));

                //self.controller_counter += 37937;
            }
            0x13 => {
                self.push_stat();

                self.controller_response_buffer.push(1);
                self.controller_response_buffer.push(1);
            }
            0x14 => {
                self.push_stat();
                self.controller_response_buffer.push(0);
                self.controller_response_buffer.push(0);
            }
            0x15 | 0x16 => {
                self.seeking = true;
                self.reading = false;
                self.playing = false;

                self.push_stat();

                self.data_busy = false;

                self.drive_mode = CdromDriveMode::Seek;
                self.next_drive_mode = CdromDriveMode::GetStat;

                self.drive_counter += match self.mode_double_speed {
                    false => 28,
                    true => 14,
                };
            }
            0x19 => {
                self.execute_test_command();
            }
            0x1a => {
                self.push_stat();

                self.second_response_mode = CdromSecondResponseMode::GetID;
                self.second_response_counter += 50;
            }
            0x1b => {
                if self.seek_unprocessed {
                    self.seeking = true;
                    self.reading = false;
                    self.playing = false;

                    self.drive_mode = CdromDriveMode::Seek;
                    self.next_drive_mode = CdromDriveMode::Read;

                    self.drive_counter += match self.mode_double_speed {
                        false => 28,
                        true => 14,
                    };
                } else {
                    self.seeking = false;
                    self.reading = true;
                    self.playing = false;

                    self.drive_mode = CdromDriveMode::Read;

                    self.drive_counter += 44100
                        / match self.mode_double_speed {
                            true => 150,
                            false => 75,
                        };
                }

                self.push_stat();
            }
            0x1e => {
                self.push_stat();

                //self.controller_counter = 81102;

                self.second_response_mode = CdromSecondResponseMode::GetStat;
                self.second_response_counter += 44100;
            }
            _ => panic!("[CDROM] Unknown command 0x{:02x}", command),
        };

        self.controller_interrupt_flags = interrupt;
    }

    fn execute_test_command(&mut self) {
        let command = self.controller_parameter_buffer.pop();

        match command {
            0x20 => {
                self.controller_response_buffer.push(0x97);
                self.controller_response_buffer.push(0x01);
                self.controller_response_buffer.push(0x10);
                self.controller_response_buffer.push(0xc2);
            }
            _ => panic!("[CDROM] [ERROR] Unknown test command 0x{:02x}", command),
        }
    }

    fn get_seek_location(&self) -> u64 {
        let mut sector = ((self.drive_seek_minute as u64) * SECTORS_PER_MINUTE)
            + ((self.drive_seek_second as u64) * SECTORS_PER_SECOND)
            + self.drive_seek_sector as u64;

        if sector >= LEAD_IN_SECTORS {
            sector -= LEAD_IN_SECTORS;
        }

        sector * BYTES_PER_SECTOR
    }

    fn get_stat(&self) -> u8 {
        let mut stat = 0;

        stat |= (self.playing as u8) << 7;
        stat |= (self.seeking as u8) << 6;
        stat |= (self.reading as u8) << 5;
        stat |= 0x2;

        stat
    }

    fn push_stat(&mut self) {
        let stat = self.get_stat();
        self.controller_response_buffer.push(stat);
    }

    pub fn busy(&self) -> bool {
        if self.controller_mode != CdromControllerMode::Idle {
            return true;
        }

        false
    }

    fn data_buffer_empty(&self) -> bool {
        let max = match self.mode_sector_size {
            false => 0x800,
            true => 0x924,
        };

        self.data_buffer_ptr >= max
    }

    pub fn read(&mut self, address: u32) -> u8 {
        use self::CdromIndex::*;

        let mut value = 0;

        match address & 0x3 {
            0 => {
                value |= (self.busy() as u8) << 7;
                value |= (!self.data_buffer_empty() as u8) << 6;
                value |= (self.response_buffer.has_data() as u8) << 5;
                value |= (self.parameter_buffer.has_space() as u8) << 4;
                value |= (self.parameter_buffer.is_empty() as u8) << 3;
                //value |= 4;
                //value |= (self. as u8) << 2;
                value |= self.index as u8;
            }
            1 => {
                value = self.response_buffer.pop();
            }
            2 => {
                value = self.read_data();
            }
            3 => match self.index {
                Index0 => value = 0xe0 | self.interrupt_enable,
                Index1 => value = 0xe0 | self.interrupt_flags,
                _ => panic!(
                    "[CDROM] [ERROR] Read from CDROM_REG_{}_{:?}",
                    address & 0x3,
                    self.index
                ),
            },
            _ => panic!(
                "[CDROM] [ERROR] Read from CDROM_REG_{}_{:?}",
                address & 0x3,
                self.index
            ),
        };

        value
    }

    fn read_data(&mut self) -> u8 {
        let offset = match self.mode_sector_size {
            false => DATA_OFFSET,
            true => ADDRESS_OFFSET,
        };

        if self.data_buffer_empty() {
            println!("[CDROM] [WARN] Reading from empty data buffer");

            let data = match self.mode_sector_size {
                false => self.data_buffer[0x810],
                true => self.data_buffer[0x92c],
            };

            self.data_buffer_ptr += 1;
            return data;
        }

        let data = self.data_buffer[self.data_buffer_ptr + offset];
        self.data_buffer_ptr += 1;

        data
    }

    pub fn read_data_half(&mut self) -> u16 {
        let h0 = self.read_data() as u16;
        let h1 = self.read_data() as u16;

        (h1 << 8) | h0
    }

    pub fn data_dma(&mut self) -> u32 {
        let b0 = self.read_data() as u32;
        let b1 = self.read_data() as u32;
        let b2 = self.read_data() as u32;
        let b3 = self.read_data() as u32;

        (b3 << 24) | (b2 << 16) | (b1 << 8) | b0
    }

    pub fn write(&mut self, address: u32, value: u8) {
        use self::CdromIndex::*;

        match address & 0x3 {
            0 => {
                self.index = match value & 0x3 {
                    0 => Index0,
                    1 => Index1,
                    2 => Index2,
                    3 => Index3,
                    _ => unreachable!(),
                };
            }
            1 => {
                match self.index {
                    Index0 => {
                        self.command = Some(value);
                    }
                    Index3 => (), // Right-CD to Right-SPU
                    _ => panic!(
                        "[CDROM] [ERROR] Write to CDROM_REG_{}_{:?}",
                        address & 0x3,
                        self.index
                    ),
                }
            }
            2 => {
                match self.index {
                    Index0 => self.parameter_buffer.push(value),
                    Index1 => self.interrupt_enable = value & 0x1f,
                    Index2 => (), // Left-CD to Left-SPU
                    Index3 => (), // Right-CD to Left-SPU
                }
            }
            3 => {
                match self.index {
                    Index0 => {
                        self.want_data = (value & 0x80) != 0;

                        if !self.want_data {
                            self.data_buffer_ptr = 0x930;
                        } else {
                            if self.data_buffer_empty() {
                                self.data_buffer_ptr = 0;
                                self.data_buffer[..0x930].clone_from_slice(&self.sector[..0x930]);
                            }
                        }
                    }
                    Index1 => {
                        self.interrupt_flags &= !(value & 0x1f);

                        if self.interrupt_flags == 0 && self.drive_interrupt_pending {
                            self.interrupt_flags = 0x1;
                            self.drive_interrupt_pending = false;
                            self.response_buffer.push(self.drive_pending_stat);
                            println!("pushing pending stat from drive");
                        }

                        self.response_buffer.clear();

                        if (value & 0x40) != 0 {
                            self.parameter_buffer.clear();
                        }
                    }
                    Index2 => (), // Left-CD to Right-SPU
                    Index3 => (), // Apply Volume Change
                }
            }
            _ => panic!(
                "[CDROM] [ERROR] Write to CDROM_REG_{}_{:?}",
                address & 0x3,
                self.index
            ),
        };
    }
}
