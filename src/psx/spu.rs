use byteorder::{LittleEndian, ByteOrder};

use queue::Queue;

#[derive(Clone, Copy, PartialEq)]
enum SpuTransferMode {
    Stop,
    ManualWrite,
    DmaWrite,
    DmaRead,
}

impl SpuTransferMode {
    fn from(value: u16) -> SpuTransferMode {
        use self::SpuTransferMode::*;

        match value & 0x3 {
            0 => Stop,
            1 => ManualWrite,
            2 => DmaWrite,
            3 => DmaRead,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy)]
struct SpuControl {
    enable: bool,
    mute: bool,
    noise_freq_shift: u16,
    noise_freq_step: u16,
    reverb_enable: bool,
    irq9_enable: bool,
    transfer_mode: SpuTransferMode,
    external_reverb: bool,
    cd_reverb: bool,
    external_enable: bool,
    cd_enable: bool,
}

impl SpuControl {
    pub fn new() -> SpuControl {
        SpuControl {
            enable: false,
            mute: false,
            noise_freq_shift: 0,
            noise_freq_step: 0,
            reverb_enable: false,
            irq9_enable: false,
            transfer_mode: SpuTransferMode::Stop,
            external_reverb: false,
            cd_reverb: false,
            external_enable: false,
            cd_enable: false,
        }
    }

    pub fn read(&self) -> u16 {
        let mut value = 0;

        value |= (self.enable as u16) << 15;
        value |= (self.mute as u16) << 14;
        value |= (self.noise_freq_shift & 0xf) << 10;
        value |= (self.noise_freq_step & 0x3) << 8;
        value |= (self.reverb_enable as u16) << 7;
        value |= (self.irq9_enable as u16) << 6;
        value |= (self.transfer_mode as u16) << 4;
        value |= (self.external_reverb as u16) << 3;
        value |= (self.cd_reverb as u16) << 2;
        value |= (self.external_enable as u16) << 1;
        value |= self.cd_enable as u16;

        value
    }

    pub fn write(&mut self, value: u16) {
        self.enable = (value & 0x8000) != 0;
        self.mute = (value & 0x4000) != 0;
        self.noise_freq_shift = (value & 0x3c00) >> 10;
        self.noise_freq_step = (value & 0x300) >> 8;
        self.reverb_enable = (value & 0x80) != 0;
        self.irq9_enable = (value & 0x40) != 0;
        self.transfer_mode = SpuTransferMode::from((value & 0x30) >> 4);
        self.external_reverb = (value & 0x8) != 0;
        self.cd_reverb = (value & 0x4) != 0;
        self.external_enable = (value & 0x2) != 0;
        self.cd_enable = (value & 0x1) != 0;
    }
}

pub struct Spu {
    sound_ram: Box<[u8]>,

    voice_volume_left: [u16; 24],
    voice_volume_right: [u16; 24],
    voice_sample_rate: [u16; 24],
    voice_start_address: [u16; 24],
    voice_adsr_low: [u16; 24],
    voice_adsr_high: [u16; 24],
    voice_adsr_volume: [u16; 24],
    voice_repeat_address: [u16; 24],

    main_volume_left: u16,
    main_volume_right: u16,

    reverb_volume_left: u16,
    reverb_volume_right: u16,

    voice_channel_key_on: u32,
    voice_channel_key_off: u32,
    voice_channel_fm: u32,
    voice_channel_noise: u32,
    voice_channel_reverb: u32,
    voice_channel_on: u32,

    control: SpuControl,

    reverb_work_area_start: u16,

    irq_address: u16,

    data_transfer_address: u16,
    current_transfer_address: u16,

    data_transfer_fifo: Queue<u16>,

    data_transfer_control: u16,

    writing_to_capture_buffer_half: bool,
    data_transfer_busy: bool,
    data_transfer_dma_read: bool,
    data_transfer_dma_write: bool,
    irq9_flag: bool,

    cd_volume_left: u16,
    cd_volume_right: u16,

    extern_volume_left: u16,
    extern_volume_right: u16,

    current_volume_left: u16,
    current_volume_right: u16,

    rev: [u16; 0x20],
}

impl Spu {
    pub fn new() -> Spu {
        Spu {
            sound_ram: vec![0; 0x80000].into_boxed_slice(),

            voice_volume_left: [0; 24],
            voice_volume_right: [0; 24],
            voice_sample_rate: [0; 24],
            voice_start_address: [0; 24],
            voice_adsr_low: [0; 24],
            voice_adsr_high: [0; 24],
            voice_adsr_volume: [0; 24],
            voice_repeat_address: [0; 24],

            main_volume_left: 0,
            main_volume_right: 0,

            reverb_volume_left: 0,
            reverb_volume_right: 0,

            voice_channel_key_on: 0,
            voice_channel_key_off: 0,
            voice_channel_fm: 0,
            voice_channel_noise: 0,
            voice_channel_reverb: 0,
            voice_channel_on: 0,

            control: SpuControl::new(),

            reverb_work_area_start: 0,

            irq_address: 0,

            data_transfer_address: 0,
            current_transfer_address: 0,

            data_transfer_fifo: Queue::<u16>::new(32),

            data_transfer_control: 0,

            writing_to_capture_buffer_half: false,
            data_transfer_busy: false,
            data_transfer_dma_read: false,
            data_transfer_dma_write: false,
            irq9_flag: false,

            cd_volume_left: 0,
            cd_volume_right: 0,

            extern_volume_left: 0,
            extern_volume_right: 0,

            current_volume_left: 0,
            current_volume_right: 0,

            rev: [0; 0x20],
        }
    }

    fn read_status(&self) -> u16 {
        let mut value = 0;
        let control = self.control.read();

        value |= (self.writing_to_capture_buffer_half as u16) << 11;
        value |= (self.data_transfer_busy as u16) << 10;
        value |= (self.data_transfer_dma_read as u16) << 9;
        value |= (self.data_transfer_dma_write as u16) << 8;
        value |= (control & 0x20) << 2;
        value |= (self.irq9_flag as u16) << 6;
        value |= control & 0x3f;

        value
    }

    pub fn read(&mut self, address: u32) -> u32 {
        match address {
            0x1f801c00...0x1f801d7f => {
                let voice = ((address - 0x1f801c00) / 0x10) as usize;

                match address & 0xf {
                    0x0 => self.voice_volume_left[voice] as u32,
                    0x2 => self.voice_volume_right[voice] as u32,
                    0x4 => self.voice_sample_rate[voice] as u32,
                    0x6 => self.voice_start_address[voice] as u32,
                    0x8 => self.voice_adsr_low[voice] as u32,
                    0xa => self.voice_adsr_high[voice] as u32,
                    0xc => self.voice_adsr_volume[voice] as u32,
                    0xe => self.voice_repeat_address[voice] as u32,
                    _ => panic!("[SPU] [ERROR] Read from unimplemented register: 0x{:08x}", address),
                }
            },
            0x1f801d80 => self.main_volume_left as u32,
            0x1f801d82 => self.main_volume_right as u32,
            0x1f801d84 => self.reverb_volume_left as u32,
            0x1f801d86 => self.reverb_volume_right as u32,
            0x1f801d88 => { println!("[SPU] [WARN] Read from KON register"); 0 },
            0x1f801d8a => { println!("[SPU] [WARN] Read from KON register"); 0 },
            0x1f801d8c => { println!("[SPU] [WARN] Read from KOFF register"); 0 },
            0x1f801d8e => { println!("[SPU] [WARN] Read from KOFF register"); 0 },
            0x1f801d90 => self.voice_channel_fm,
            0x1f801d92 => self.voice_channel_fm,
            0x1f801d94 => self.voice_channel_noise,
            0x1f801d96 => self.voice_channel_noise,
            0x1f801d98 => self.voice_channel_reverb,
            0x1f801d9a => self.voice_channel_reverb,
            0x1f801d9c => self.voice_channel_on,
            0x1f801d9e => self.voice_channel_on,
            0x1f801da2 => self.reverb_work_area_start as u32,
            0x1f801da6 => self.data_transfer_address as u32,
            0x1f801da8 => { println!("[SPU] [WARN] Read from data transfer FIFO"); 0 },
            0x1f801daa => self.control.read() as u32,
            0x1f801dac => (self.data_transfer_control << 1) as u32,
            0x1f801dae => self.read_status() as u32,
            0x1f801db0 => self.cd_volume_left as u32,
            0x1f801db2 => self.cd_volume_right as u32,
            0x1f801db4 => self.extern_volume_left as u32,
            0x1f801db6 => self.extern_volume_right as u32,
            0x1f801db8 => self.current_volume_left as u32,
            0x1f801dba => self.current_volume_right as u32,
            0x1f801dc0...0x1f801dff => {
                let register = ((address - 0x1f801dc0) / 2) as usize;
                self.rev[register] as u32
            },
            0x1f801e00...0x1f801fff => {
                0xffffffff
            },
            _ => panic!("[SPU] [ERROR] Read from unimplemented register: 0x{:08x}", address),
        }
    }

    pub fn write(&mut self, address: u32, value: u32) {
        match address {
            0x1f801c00...0x1f801d7f => {
                let voice = ((address - 0x1f801c00) / 0x10) as usize;

                match address & 0xf {
                    0x0 => self.voice_volume_left[voice] = value as u16,
                    0x2 => self.voice_volume_right[voice] = value as u16, 
                    0x4 => self.voice_sample_rate[voice] = value as u16, 
                    0x6 => self.voice_start_address[voice] = value as u16, 
                    0x8 => self.voice_adsr_low[voice] = value as u16, 
                    0xa => self.voice_adsr_high[voice] = value as u16, 
                    0xc => self.voice_adsr_volume[voice] = value as u16, 
                    0xe => self.voice_repeat_address[voice] = value as u16, 
                    _ => panic!("[SPU] [ERROR] Write to unimplemented register: 0x{:08x}", address),
                };
            },
            0x1f801d80 => self.main_volume_left = value as u16,
            0x1f801d82 => self.main_volume_right = value as u16,
            0x1f801d84 => self.reverb_volume_left = value as u16,
            0x1f801d86 => self.reverb_volume_right = value as u16,
            0x1f801d88 => self.voice_channel_key_on = value,
            0x1f801d8a => self.voice_channel_key_on = value,
            0x1f801d8c => self.voice_channel_key_off = value,
            0x1f801d8e => self.voice_channel_key_off = value,
            0x1f801d90 => self.voice_channel_fm = value,
            0x1f801d92 => self.voice_channel_fm = value,
            0x1f801d94 => self.voice_channel_noise = value,
            0x1f801d96 => self.voice_channel_noise = value,
            0x1f801d98 => self.voice_channel_reverb = value,
            0x1f801d9a => self.voice_channel_reverb = value,
            0x1f801d9c => println!("[SPU] [WARN] Write to ENDX register"),
            0x1f801d9e => println!("[SPU] [WARN] Write to ENDX register"),
            0x1f801da2 => self.reverb_work_area_start = value as u16,
            0x1f801da4 => self.irq_address = value as u16,
            0x1f801da6 => {
                self.data_transfer_address = value as u16;
                self.current_transfer_address = self.data_transfer_address * 8;
            },
            0x1f801da8 => self.data_transfer_fifo.push(value as u16),
            0x1f801daa => {
                self.control.write(value as u16);

                if self.control.transfer_mode == SpuTransferMode::ManualWrite {
                    while self.data_transfer_fifo.has_data() {
                        let data = self.data_transfer_fifo.pop();
                        let address = self.current_transfer_address as usize;
                        let slice = &mut self.sound_ram[address..];

                        LittleEndian::write_u16(slice, data);

                        self.current_transfer_address = self.current_transfer_address.wrapping_add(2);
                    }
                }
            },
            0x1f801dac => self.data_transfer_control = (value << 1) as u16,
            0x1f801dae => println!("[SPU] [WARN] Write to SPUSTAT"),
            0x1f801db0 => self.cd_volume_left = value as u16,
            0x1f801db2 => self.cd_volume_right = value as u16,
            0x1f801db4 => self.extern_volume_left = value as u16,
            0x1f801db6 => self.extern_volume_right = value as u16,
            0x1f801db8 => self.current_volume_left = value as u16,
            0x1f801dba => self.current_volume_right = value as u16,
            0x1f801dc0...0x1f801dff => {
                let register = ((address - 0x1f801dc0) / 2) as usize;
                self.rev[register] = value as u16;
            },
            _ => panic!("[SPU] [ERROR] Write to unimplemented register: 0x{:08x}", address),
        };
    }
    
    pub fn dma_write(&mut self, value: u32) {
        let address = self.current_transfer_address as usize;
        let slice = &mut self.sound_ram[address..];

        LittleEndian::write_u32(slice, value);

        self.current_transfer_address = self.current_transfer_address.wrapping_add(4);
    }
}