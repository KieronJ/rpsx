use std::cmp;

#[derive(Clone, Copy)]
struct GteMatrix {
    m11: i16,
    m12: i16,
    m13: i16,
    m21: i16,
    m22: i16,
    m23: i16,
    m31: i16,
    m32: i16,
    m33: i16,
}

impl GteMatrix {
    fn new() -> GteMatrix {
        GteMatrix {
            m11: 0,
            m12: 0,
            m13: 0,
            m21: 0,
            m22: 0,
            m23: 0,
            m31: 0,
            m32: 0,
            m33: 0,
        }
    }
}

#[derive(Clone, Copy)]
struct GteVector2 {
    x: i16,
    y: i16,
}

impl GteVector2 {
    fn new() -> GteVector2 {
        GteVector2 { x: 0, y: 0 }
    }
}

#[derive(Clone, Copy)]
struct GteVector3 {
    x: i16,
    y: i16,
    z: i16,
}

impl GteVector3 {
    fn new() -> GteVector3 {
        GteVector3 { x: 0, y: 0, z: 0 }
    }
}

#[derive(Clone, Copy)]
struct GteVector3_32 {
    x: i32,
    y: i32,
    z: i32,
}

impl GteVector3_32 {
    fn new() -> GteVector3_32 {
        GteVector3_32 { x: 0, y: 0, z: 0 }
    }
}

#[derive(Clone, Copy)]
struct GteRgb {
    r: u8,
    g: u8,
    b: u8,
    c: u8,
}

impl GteRgb {
    fn new() -> GteRgb {
        GteRgb {
            r: 0,
            g: 0,
            b: 0,
            c: 0,
        }
    }
}

const UNR_TABLE: [u8; 0x101] = [
    0xFF, 0xFD, 0xFB, 0xF9, 0xF7, 0xF5, 0xF3, 0xF1, 0xEF, 0xEE, 0xEC, 0xEA, 0xE8, 0xE6, 0xE4, 0xE3,
    0xE1, 0xDF, 0xDD, 0xDC, 0xDA, 0xD8, 0xD6, 0xD5, 0xD3, 0xD1, 0xD0, 0xCE, 0xCD, 0xCB, 0xC9, 0xC8,
    0xC6, 0xC5, 0xC3, 0xC1, 0xC0, 0xBE, 0xBD, 0xBB, 0xBA, 0xB8, 0xB7, 0xB5, 0xB4, 0xB2, 0xB1, 0xB0,
    0xAE, 0xAD, 0xAB, 0xAA, 0xA9, 0xA7, 0xA6, 0xA4, 0xA3, 0xA2, 0xA0, 0x9F, 0x9E, 0x9C, 0x9B, 0x9A,
    0x99, 0x97, 0x96, 0x95, 0x94, 0x92, 0x91, 0x90, 0x8F, 0x8D, 0x8C, 0x8B, 0x8A, 0x89, 0x87, 0x86,
    0x85, 0x84, 0x83, 0x82, 0x81, 0x7F, 0x7E, 0x7D, 0x7C, 0x7B, 0x7A, 0x79, 0x78, 0x77, 0x75, 0x74,
    0x73, 0x72, 0x71, 0x70, 0x6F, 0x6E, 0x6D, 0x6C, 0x6B, 0x6A, 0x69, 0x68, 0x67, 0x66, 0x65, 0x64,
    0x63, 0x62, 0x61, 0x60, 0x5F, 0x5E, 0x5D, 0x5D, 0x5C, 0x5B, 0x5A, 0x59, 0x58, 0x57, 0x56, 0x55,
    0x54, 0x53, 0x53, 0x52, 0x51, 0x50, 0x4F, 0x4E, 0x4D, 0x4D, 0x4C, 0x4B, 0x4A, 0x49, 0x48, 0x48,
    0x47, 0x46, 0x45, 0x44, 0x43, 0x43, 0x42, 0x41, 0x40, 0x3F, 0x3F, 0x3E, 0x3D, 0x3C, 0x3C, 0x3B,
    0x3A, 0x39, 0x39, 0x38, 0x37, 0x36, 0x36, 0x35, 0x34, 0x33, 0x33, 0x32, 0x31, 0x31, 0x30, 0x2F,
    0x2E, 0x2E, 0x2D, 0x2C, 0x2C, 0x2B, 0x2A, 0x2A, 0x29, 0x28, 0x28, 0x27, 0x26, 0x26, 0x25, 0x24,
    0x24, 0x23, 0x22, 0x22, 0x21, 0x20, 0x20, 0x1F, 0x1E, 0x1E, 0x1D, 0x1D, 0x1C, 0x1B, 0x1B, 0x1A,
    0x19, 0x19, 0x18, 0x18, 0x17, 0x16, 0x16, 0x15, 0x15, 0x14, 0x14, 0x13, 0x12, 0x12, 0x11, 0x11,
    0x10, 0x0F, 0x0F, 0x0E, 0x0E, 0x0D, 0x0D, 0x0C, 0x0C, 0x0B, 0x0A, 0x0A, 0x09, 0x09, 0x08, 0x08,
    0x07, 0x07, 0x06, 0x06, 0x05, 0x05, 0x04, 0x04, 0x03, 0x03, 0x02, 0x02, 0x01, 0x01, 0x00, 0x00,
    0x00 
];

pub struct Gte {
    sf: usize,
    mx: usize,
    sv: usize,
    cv: usize,
    lm: bool,

    // CONTROL REGISTERS
    rotation: GteMatrix,

    tr: GteVector3_32,

    light: GteMatrix,

    bk: GteVector3_32,

    colour: GteMatrix,

    fc: GteVector3_32,

    ofx: i32,
    ofy: i32,

    h: u16,

    dqa: i16,
    dqb: i32,

    zsf3: i16,
    zsf4: i16,

    flags: u32,

    // DATA REGISTERS
    v: [GteVector3; 3],

    rgb: GteRgb,

    otz: u16,

    ir: [i16; 4],

    sxy_fifo: [GteVector2; 3],
    sz_fifo: [u16; 4],

    rgb_fifo: [GteRgb; 3],

    res1: u32,

    mac: [i32; 4],

    lzcs: i32,
    lzcr: i32,
}

impl Gte {
    pub fn new() -> Gte {
        Gte {
            sf: 0,
            mx: 0,
            sv: 0,
            cv: 0,
            lm: false,

            // CONTROL REGISTERS
            rotation: GteMatrix::new(),

            tr: GteVector3_32::new(),

            light: GteMatrix::new(),

            bk: GteVector3_32::new(),

            colour: GteMatrix::new(),

            fc: GteVector3_32::new(),

            ofx: 0,
            ofy: 0,

            h: 0,

            dqa: 0,
            dqb: 0,

            zsf3: 0,
            zsf4: 0,

            flags: 0,

            // DATA REGISTERS
            v: [GteVector3::new(); 3],

            rgb: GteRgb::new(),

            otz: 0,

            ir: [0; 4],

            sxy_fifo: [GteVector2::new(); 3],
            sz_fifo: [0; 4],

            rgb_fifo: [GteRgb::new(); 3],

            res1: 0,

            mac: [0; 4],

            lzcs: 0,
            lzcr: 0,
        }
    }

    pub fn execute(&mut self, command: u32) {
        self.sf = match (command & 0x8_0000) != 0 {
            true => 12,
            false => 0,
        };

        self.mx = ((command >> 17) & 0x3) as usize;
        self.sv = ((command >> 15) & 0x3) as usize;
        self.cv = ((command >> 13) & 0x3) as usize;

        self.lm = (command & 0x400) != 0;

        let opcode = command & 0x3f;

        self.flags = 0;

        match opcode {
            0x01 => self.command_rtps(),
            0x06 => self.command_nclip(),
            0x0c => self.command_op(),
            0x10 => self.command_dpcs(),
            0x11 => self.command_intpl(),
            0x12 => self.command_mvmva(),
            0x13 => self.command_ncds(),
            0x14 => self.command_cdp(),
            0x16 => self.command_ncdt(),
            0x1b => self.command_nccs(),
            0x1c => self.command_cc(),
            0x1e => self.command_ncs(),
            0x20 => self.command_nct(),
            0x28 => self.command_sqr(),
            0x29 => self.command_dcpl(),
            0x2a => self.command_dpct(),
            0x2d => self.command_avsz3(),
            0x2e => self.command_avsz4(),
            0x30 => self.command_rtpt(),
            0x3d => self.command_gpf(),
            0x3e => self.command_gpl(),
            0x3f => self.command_ncct(),
            _ => panic!("[GTE] [ERROR] Unimplemented GTE function 0x{:02x}", opcode),
        };

        if (self.flags & 0x7f87e000) != 0 {
            self.flags |= 0x8000_0000;
        }
    }

    fn command_rtps(&mut self) {
        self.rtp(0, true);
    }

    fn command_nclip(&mut self) {
        let p = (self.sxy_fifo[0].x as i64 * self.sxy_fifo[1].y as i64)
            + (self.sxy_fifo[1].x as i64 * self.sxy_fifo[2].y as i64)
            + (self.sxy_fifo[2].x as i64 * self.sxy_fifo[0].y as i64)
            - (self.sxy_fifo[0].x as i64 * self.sxy_fifo[2].y as i64)
            - (self.sxy_fifo[1].x as i64 * self.sxy_fifo[0].y as i64)
            - (self.sxy_fifo[2].x as i64 * self.sxy_fifo[1].y as i64);

        self.mac[0] = self.f(p) as i32;
    }

    fn command_op(&mut self) {
        let lm = self.lm;

        let ir1 = self.ir[1] as i64;
        let ir2 = self.ir[2] as i64;
        let ir3 = self.ir[3] as i64;

        let d1 = self.rotation.m11 as i64;
        let d2 = self.rotation.m22 as i64;
        let d3 = self.rotation.m33 as i64;

        self.mac[1] = (self.a(1, ir3 * d2 - ir2 * d3) >> self.sf) as i32;
        self.mac[2] = (self.a(2, ir1 * d3 - ir3 * d1) >> self.sf) as i32;
        self.mac[3] = (self.a(3, ir2 * d1 - ir1 * d2) >> self.sf) as i32;

        let mac1 = self.mac[1];
        let mac2 = self.mac[2];
        let mac3 = self.mac[3];

        self.ir[1] = self.lm_b(1, mac1, lm);
        self.ir[2] = self.lm_b(2, mac2, lm);
        self.ir[3] = self.lm_b(3, mac3, lm);
    }

    fn command_dpcs(&mut self) {
        self.dpc(false);
    }

    fn command_intpl(&mut self) {
        let lm = self.lm;

        let prev_ir1 = (self.ir[1] as i64) << 12;
        let prev_ir2 = (self.ir[2] as i64) << 12;
        let prev_ir3 = (self.ir[3] as i64) << 12;

        let rfc = (self.fc.x as i64) << 12;
        let gfc = (self.fc.y as i64) << 12;
        let bfc = (self.fc.z as i64) << 12;

        self.mac[1] = (self.a(1, rfc - prev_ir1) >> self.sf) as i32;
        self.mac[2] = (self.a(2, gfc - prev_ir2) >> self.sf) as i32;
        self.mac[3] = (self.a(3, bfc - prev_ir3) >> self.sf) as i32;

        let mac1 = self.mac[1];
        let mac2 = self.mac[2];
        let mac3 = self.mac[3];

        self.ir[1] = self.lm_b(1, mac1, false);
        self.ir[2] = self.lm_b(2, mac2, false);
        self.ir[3] = self.lm_b(3, mac3, false);

        let ir0 = self.ir[0] as i64;
        let ir1 = self.ir[1] as i64;
        let ir2 = self.ir[2] as i64;
        let ir3 = self.ir[3] as i64;

        self.mac[1] = (self.a(1, prev_ir1 + ir1 * ir0) >> self.sf) as i32;
        self.mac[2] = (self.a(2, prev_ir2 + ir2 * ir0) >> self.sf) as i32;
        self.mac[3] = (self.a(3, prev_ir3 + ir3 * ir0) >> self.sf) as i32;

        let mac1 = self.mac[1];
        let mac2 = self.mac[2];
        let mac3 = self.mac[3];

        self.ir[1] = self.lm_b(1, mac1, lm);
        self.ir[2] = self.lm_b(2, mac2, lm);
        self.ir[3] = self.lm_b(3, mac3, lm);

        let r = self.lm_c(1, mac1 >> 4);
        let g = self.lm_c(2, mac2 >> 4);
        let b = self.lm_c(3, mac3 >> 4);
        let c = self.rgb.c;
        self.push_rgb(r, g, b, c);
    }

    fn command_dpct(&mut self) {
        self.dpc(true);
        self.dpc(true);
        self.dpc(true);
    }

    fn command_mvmva(&mut self) {
        let sf = self.sf;
        let lm = self.lm;

        let mx = match self.mx {
            0 => self.rotation,
            1 => self.light,
            2 => self.colour,
            3 => {
                let mut m = GteMatrix::new();

                m.m11 = -((self.rgb.r as i16) << 4);
                m.m12 = (self.rgb.r as i16) << 4;
                m.m13 = self.ir[0];
                m.m21 = self.rotation.m13;
                m.m22 = self.rotation.m13;
                m.m23 = self.rotation.m13;
                m.m31 = self.rotation.m22;
                m.m32 = self.rotation.m22;
                m.m33 = self.rotation.m22;

                m
            }
            _ => unreachable!(),
        };

        let mx11 = mx.m11 as i64;
        let mx12 = mx.m12 as i64;
        let mx13 = mx.m13 as i64;
        let mx21 = mx.m21 as i64;
        let mx22 = mx.m22 as i64;
        let mx23 = mx.m23 as i64;
        let mx31 = mx.m31 as i64;
        let mx32 = mx.m32 as i64;
        let mx33 = mx.m33 as i64;

        let (v1, v2, v3) = match self.sv {
            0 => (self.v[0].x, self.v[0].y, self.v[0].z),
            1 => (self.v[1].x, self.v[1].y, self.v[1].z),
            2 => (self.v[2].x, self.v[2].y, self.v[2].z),
            3 => (self.ir[1], self.ir[2], self.ir[3]),
            _ => unreachable!(),
        };

        let vx = v1 as i64;
        let vy = v2 as i64;
        let vz = v3 as i64;

        let (tx, ty, tz) = match self.cv {
            0 => (self.tr.x, self.tr.y, self.tr.z),
            1 => (self.bk.x, self.bk.y, self.bk.z),
            2 => (self.fc.x, self.fc.y, self.fc.z),
            3 => (0, 0, 0),
            _ => unreachable!(),
        };

        let tr_x = (tx as i64) << 12;
        let tr_y = (ty as i64) << 12;
        let tr_z = (tz as i64) << 12;

        let mut temp = [0; 3];

        temp[0] = self.a(1, tr_x + mx11 * vx);
        temp[1] = self.a(2, tr_y + mx21 * vx);
        temp[2] = self.a(3, tr_z + mx31 * vx);

        if self.cv == 2 {
            self.lm_b(1, (temp[0] >> sf) as i32, false);
            self.lm_b(2, (temp[1] >> sf) as i32, false);
            self.lm_b(3, (temp[2] >> sf) as i32, false);

            temp[0] = 0;
            temp[1] = 0;
            temp[2] = 0;
        }

        temp[0] = self.a(1, temp[0] + mx12 * vy);
        temp[1] = self.a(2, temp[1] + mx22 * vy);
        temp[2] = self.a(3, temp[2] + mx32 * vy);

        temp[0] = self.a(1, temp[0] + mx13 * vz);
        temp[1] = self.a(2, temp[1] + mx23 * vz);
        temp[2] = self.a(3, temp[2] + mx33 * vz);

        self.mac[1] = (temp[0] >> sf) as i32;
        self.mac[2] = (temp[1] >> sf) as i32;
        self.mac[3] = (temp[2] >> sf) as i32;

        let mac1 = self.mac[1];
        let mac2 = self.mac[2];
        let mac3 = self.mac[3];

        self.ir[1] = self.lm_b(1, mac1, lm);
        self.ir[2] = self.lm_b(2, mac2, lm);
        self.ir[3] = self.lm_b(3, mac3, lm);
    }

    fn command_ncds(&mut self) {
        self.ncd(0);
    }

    fn command_nccs(&mut self) {
        self.ncc(0);
    }

    fn command_cc(&mut self) {
        let lm = self.lm;

        let c11 = self.colour.m11 as i64;
        let c12 = self.colour.m12 as i64;
        let c13 = self.colour.m13 as i64;
        let c21 = self.colour.m21 as i64;
        let c22 = self.colour.m22 as i64;
        let c23 = self.colour.m23 as i64;
        let c31 = self.colour.m31 as i64;
        let c32 = self.colour.m32 as i64;
        let c33 = self.colour.m33 as i64;

        let rbk = (self.bk.x as i64) << 12;
        let gbk = (self.bk.y as i64) << 12;
        let bbk = (self.bk.z as i64) << 12;

        let ir1 = self.ir[1] as i64;
        let ir2 = self.ir[2] as i64;
        let ir3 = self.ir[3] as i64;

        let mut temp = [0; 3];

        temp[0] = self.a(1, rbk + c11 * ir1);
        temp[1] = self.a(2, gbk + c21 * ir1);
        temp[2] = self.a(3, bbk + c31 * ir1);

        temp[0] = self.a(1, temp[0] + c12 * ir2);
        temp[1] = self.a(2, temp[1] + c22 * ir2);
        temp[2] = self.a(3, temp[2] + c32 * ir2);

        temp[0] = self.a(1, temp[0] + c13 * ir3);
        temp[1] = self.a(2, temp[1] + c23 * ir3);
        temp[2] = self.a(3, temp[2] + c33 * ir3);

        self.mac[1] = (temp[0] >> self.sf) as i32;
        self.mac[2] = (temp[1] >> self.sf) as i32;
        self.mac[3] = (temp[2] >> self.sf) as i32;

        let mac1 = self.mac[1];
        let mac2 = self.mac[2];
        let mac3 = self.mac[3];

        self.ir[1] = self.lm_b(1, mac1, lm);
        self.ir[2] = self.lm_b(2, mac2, lm);
        self.ir[3] = self.lm_b(3, mac3, lm);

        let r = (self.rgb.r as i64) << 4;
        let g = (self.rgb.g as i64) << 4;
        let b = (self.rgb.b as i64) << 4;

        let ir1 = self.ir[1] as i64;
        let ir2 = self.ir[2] as i64;
        let ir3 = self.ir[3] as i64;

        self.mac[1] = (self.a(1, r * ir1) >> self.sf) as i32;
        self.mac[2] = (self.a(2, g * ir2) >> self.sf) as i32;
        self.mac[3] = (self.a(3, b * ir3) >> self.sf) as i32;

        let mac1 = self.mac[1];
        let mac2 = self.mac[2];
        let mac3 = self.mac[3];

        self.ir[1] = self.lm_b(1, mac1, lm);
        self.ir[2] = self.lm_b(2, mac2, lm);
        self.ir[3] = self.lm_b(3, mac3, lm);

        let r = self.lm_c(1, mac1 >> 4);
        let g = self.lm_c(2, mac2 >> 4);
        let b = self.lm_c(3, mac3 >> 4);
        let c = self.rgb.c;
        self.push_rgb(r, g, b, c);
    }

    fn command_cdp(&mut self) {
        let lm = self.lm;

        let c11 = self.colour.m11 as i64;
        let c12 = self.colour.m12 as i64;
        let c13 = self.colour.m13 as i64;
        let c21 = self.colour.m21 as i64;
        let c22 = self.colour.m22 as i64;
        let c23 = self.colour.m23 as i64;
        let c31 = self.colour.m31 as i64;
        let c32 = self.colour.m32 as i64;
        let c33 = self.colour.m33 as i64;

        let rbk = (self.bk.x as i64) << 12;
        let gbk = (self.bk.y as i64) << 12;
        let bbk = (self.bk.z as i64) << 12;

        let ir0 = self.ir[0] as i64;
        let ir1 = self.ir[1] as i64;
        let ir2 = self.ir[2] as i64;
        let ir3 = self.ir[3] as i64;

        let mut temp = [0; 3];

        temp[0] = self.a(1, rbk + c11 * ir1);
        temp[1] = self.a(2, gbk + c21 * ir1);
        temp[2] = self.a(3, bbk + c31 * ir1);

        temp[0] = self.a(1, temp[0] + c12 * ir2);
        temp[1] = self.a(2, temp[1] + c22 * ir2);
        temp[2] = self.a(3, temp[2] + c32 * ir2);

        temp[0] = self.a(1, temp[0] + c13 * ir3);
        temp[1] = self.a(2, temp[1] + c23 * ir3);
        temp[2] = self.a(3, temp[2] + c33 * ir3);

        self.mac[1] = (temp[0] >> self.sf) as i32;
        self.mac[2] = (temp[1] >> self.sf) as i32;
        self.mac[3] = (temp[2] >> self.sf) as i32;

        let mac1 = self.mac[1];
        let mac2 = self.mac[2];
        let mac3 = self.mac[3];

        self.ir[1] = self.lm_b(1, mac1, lm);
        self.ir[2] = self.lm_b(2, mac2, lm);
        self.ir[3] = self.lm_b(3, mac3, lm);

        let lm = self.lm;

        let ir1 = self.ir[1] as i64;
        let ir2 = self.ir[2] as i64;
        let ir3 = self.ir[3] as i64;

        let rfc = (self.fc.x as i64) << 12;
        let gfc = (self.fc.y as i64) << 12;
        let bfc = (self.fc.z as i64) << 12;

        let r = (self.rgb.r as i64) << 4;
        let g = (self.rgb.g as i64) << 4;
        let b = (self.rgb.b as i64) << 4;

        self.mac[1] = (self.a(1, rfc - r * ir1) >> self.sf) as i32;
        self.mac[2] = (self.a(2, gfc - g * ir2) >> self.sf) as i32;
        self.mac[3] = (self.a(3, bfc - b * ir3) >> self.sf) as i32;

        let mac1 = self.mac[1];
        let mac2 = self.mac[2];
        let mac3 = self.mac[3];

		let lm1 = self.lm_b(1, mac1, false) as i64;
		let lm2 = self.lm_b(2, mac2, false) as i64;
		let lm3 = self.lm_b(3, mac3, false) as i64;
		
        self.mac[1] = (self.a(1, r * ir1 + ir0 * lm1) >> self.sf) as i32;
        self.mac[2] = (self.a(2, g * ir2 + ir0 * lm2) >> self.sf) as i32;
        self.mac[3] = (self.a(3, b * ir3 + ir0 * lm3) >> self.sf) as i32;

        let mac1 = self.mac[1];
        let mac2 = self.mac[2];
        let mac3 = self.mac[3];

        self.ir[1] = self.lm_b(1, mac1, lm);
        self.ir[2] = self.lm_b(2, mac2, lm);
        self.ir[3] = self.lm_b(3, mac3, lm);

        let r = self.lm_c(1, mac1 >> 4);
        let g = self.lm_c(2, mac2 >> 4);
        let b = self.lm_c(3, mac3 >> 4);
        let c = self.rgb.c;
        self.push_rgb(r, g, b, c);
    }

    fn command_ncdt(&mut self) {
        self.ncd(0);
        self.ncd(1);
        self.ncd(2);
    }

    fn command_ncs(&mut self) {
        self.nc(0);
    }

    fn command_nct(&mut self) {
        self.nc(0);
        self.nc(1);
        self.nc(2);
    }

    fn command_sqr(&mut self) {
        let lm = self.lm;

        let ir1 = self.ir[1] as i64;
        let ir2 = self.ir[2] as i64;
        let ir3 = self.ir[3] as i64;

        self.mac[1] = (self.a(1, ir1 * ir1) >> self.sf) as i32;
        self.mac[2] = (self.a(2, ir2 * ir2) >> self.sf) as i32;
        self.mac[3] = (self.a(3, ir3 * ir3) >> self.sf) as i32;

        let mac1 = self.mac[1];
        let mac2 = self.mac[2];
        let mac3 = self.mac[3];

        self.ir[1] = self.lm_b(1, mac1, lm);
        self.ir[2] = self.lm_b(2, mac2, lm);
        self.ir[3] = self.lm_b(3, mac3, lm);
    }

    fn command_dcpl(&mut self) {
        let lm = self.lm;

        let ir0 = self.ir[0] as i64;
        let ir1 = self.ir[1] as i64;
        let ir2 = self.ir[2] as i64;
        let ir3 = self.ir[3] as i64;

        let rfc = (self.fc.x as i64) << 12;
        let gfc = (self.fc.y as i64) << 12;
        let bfc = (self.fc.z as i64) << 12;

        let r = (self.rgb.r as i64) << 4;
        let g = (self.rgb.g as i64) << 4;
        let b = (self.rgb.b as i64) << 4;

        self.mac[1] = (self.a(1, rfc - r * ir1) >> self.sf) as i32;
        self.mac[2] = (self.a(2, gfc - g * ir2) >> self.sf) as i32;
        self.mac[3] = (self.a(3, bfc - b * ir3) >> self.sf) as i32;

        let mac1 = self.mac[1];
        let mac2 = self.mac[2];
        let mac3 = self.mac[3];

		let lm1 = self.lm_b(1, mac1, false) as i64;
		let lm2 = self.lm_b(2, mac2, false) as i64;
		let lm3 = self.lm_b(3, mac3, false) as i64;
		
        self.mac[1] = (self.a(1, r * ir1 + ir0 * lm1) >> self.sf) as i32;
        self.mac[2] = (self.a(2, g * ir2 + ir0 * lm2) >> self.sf) as i32;
        self.mac[3] = (self.a(3, b * ir3 + ir0 * lm3) >> self.sf) as i32;

        let mac1 = self.mac[1];
        let mac2 = self.mac[2];
        let mac3 = self.mac[3];

        self.ir[1] = self.lm_b(1, mac1, lm);
        self.ir[2] = self.lm_b(2, mac2, lm);
        self.ir[3] = self.lm_b(3, mac3, lm);

        let r = self.lm_c(1, mac1 >> 4);
        let g = self.lm_c(2, mac2 >> 4);
        let b = self.lm_c(3, mac3 >> 4);
        let c = self.rgb.c;
        self.push_rgb(r, g, b, c);
    }

    fn command_avsz3(&mut self) {
        let sz1 = self.sz_fifo[1] as i64;
        let sz2 = self.sz_fifo[2] as i64;
        let sz3 = self.sz_fifo[3] as i64;

        let average = (self.zsf3 as i64) * (sz1 + sz2 + sz3);

        self.mac[0] = self.f(average) as i32;
        self.otz = self.lm_d(average >> 12);
    }

    fn command_avsz4(&mut self) {
        let sz0 = self.sz_fifo[0] as i64;
        let sz1 = self.sz_fifo[1] as i64;
        let sz2 = self.sz_fifo[2] as i64;
        let sz3 = self.sz_fifo[3] as i64;

        let average = (self.zsf4 as i64) * (sz0 + sz1 + sz2 + sz3);

        self.mac[0] = self.f(average) as i32;
        self.otz = self.lm_d(average >> 12);
    }

    fn command_rtpt(&mut self) {
        self.rtp(0, false);
        self.rtp(1, false);
        self.rtp(2, true);
    }

    fn command_gpf(&mut self) {
        let lm = self.lm;

        let ir0 = self.ir[0] as i64;
        let ir1 = self.ir[1] as i64;
        let ir2 = self.ir[2] as i64;
        let ir3 = self.ir[3] as i64;

        self.mac[1] = (self.a(1, ir0 * ir1) as i32) >> self.sf;
        self.mac[2] = (self.a(2, ir0 * ir2) as i32) >> self.sf;
        self.mac[3] = (self.a(3, ir0 * ir3) as i32) >> self.sf;

        let mac1 = self.mac[1];
        let mac2 = self.mac[2];
        let mac3 = self.mac[3];

        self.ir[1] = self.lm_b(1, mac1, lm);
        self.ir[2] = self.lm_b(2, mac2, lm);
        self.ir[3] = self.lm_b(3, mac3, lm);

        let r = self.lm_c(1, mac1 >> 4);
        let g = self.lm_c(2, mac2 >> 4);
        let b = self.lm_c(3, mac3 >> 4);
        let c = self.rgb.c;
        self.push_rgb(r, g, b, c);
    }

    fn command_gpl(&mut self) {
        let lm = self.lm;

        let mac1 = (self.mac[1] as i64) << self.sf;
        let mac2 = (self.mac[2] as i64) << self.sf;
        let mac3 = (self.mac[3] as i64) << self.sf;

        let ir0 = self.ir[0] as i64;
        let ir1 = self.ir[1] as i64;
        let ir2 = self.ir[2] as i64;
        let ir3 = self.ir[3] as i64;

        self.mac[1] = (self.a(1, ir0 * ir1 + mac1) >> self.sf) as i32;
        self.mac[2] = (self.a(2, ir0 * ir2 + mac2) >> self.sf) as i32;
        self.mac[3] = (self.a(3, ir0 * ir3 + mac3) >> self.sf) as i32;

        let mac1 = self.mac[1];
        let mac2 = self.mac[2];
        let mac3 = self.mac[3];

        self.ir[1] = self.lm_b(1, mac1, lm);
        self.ir[2] = self.lm_b(2, mac2, lm);
        self.ir[3] = self.lm_b(3, mac3, lm);

        let r = self.lm_c(1, mac1 >> 4);
        let g = self.lm_c(2, mac2 >> 4);
        let b = self.lm_c(3, mac3 >> 4);
        let c = self.rgb.c;
        self.push_rgb(r, g, b, c);
    }

    fn command_ncct(&mut self) {
        self.ncc(0);
        self.ncc(1);
        self.ncc(2);
    }

    fn rtp(&mut self, index: usize, dq: bool) {
        let sf = self.sf;
        let lm = self.lm;

        let tr_x = (self.tr.x as i64) << 12;
        let tr_y = (self.tr.y as i64) << 12;
        let mut tr_z = (self.tr.z as i64) << 12;

        let r11 = self.rotation.m11 as i64;
        let r12 = self.rotation.m12 as i64;
        let r13 = self.rotation.m13 as i64;
        let r21 = self.rotation.m21 as i64;
        let r22 = self.rotation.m22 as i64;
        let r23 = self.rotation.m23 as i64;
        let r31 = self.rotation.m31 as i64;
        let r32 = self.rotation.m32 as i64;
        let r33 = self.rotation.m33 as i64;

        let vx = self.v[index].x as i64;
        let vy = self.v[index].y as i64;
        let vz = self.v[index].z as i64;

        let mut temp = [0; 3];

        temp[0] = self.a(1, tr_x + r11 * vx);
        temp[1] = self.a(2, tr_y + r21 * vx);
        temp[2] = self.a(3, tr_z + r31 * vx);

        temp[0] = self.a(1, temp[0] + r12 * vy);
        temp[1] = self.a(2, temp[1] + r22 * vy);
        temp[2] = self.a(3, temp[2] + r32 * vy);

        temp[0] = self.a(1, temp[0] + r13 * vz);
        temp[1] = self.a(2, temp[1] + r23 * vz);
        temp[2] = self.a(3, temp[2] + r33 * vz);

        self.mac[1] = (temp[0] >> self.sf) as i32;
        self.mac[2] = (temp[1] >> self.sf) as i32;
        tr_z = temp[2];
        self.mac[3] = (tr_z >> sf) as i32;

        let zs = tr_z >> 12;

        let mac1 = self.mac[1];
        let mac2 = self.mac[2];
        let mac3 = self.mac[3];

        self.ir[1] = self.lm_b(1, mac1, lm);
        self.ir[2] = self.lm_b(2, mac2, lm);
        self.ir[3] = self.lm_b_z(mac3, zs, lm);

        let sz3 = self.lm_d(zs);

        self.push_sz(sz3);

        let h_div_sz;

        if sz3 > (self.h / 2) {
            h_div_sz = Gte::divide(self.h, sz3);
        } else {
            self.flags |= 0x2_0000;
            h_div_sz = 0x1_ffff;
        }

        let ir1 = self.ir[1] as i64;
        let ir2 = self.ir[2] as i64;

        let sx2 = self.ofx as i64 + ir1 * h_div_sz as i64;
        let sx2_f = self.f(sx2) >> 16;
        let sx2_f_g = self.lm_g(1, sx2_f as i32);

        self.push_sx(sx2_f_g);

        let sy2 = self.ofy as i64 + ir2 * h_div_sz as i64;
        let sy2_f = self.f(sy2) >> 16;
        let sy2_f_g = self.lm_g(2, sy2_f as i32);

        self.push_sy(sy2_f_g);

        if dq {
            let depth = self.dqb as i64 + self.dqa as i64 * h_div_sz as i64;
            self.mac[0] = self.f(depth) as i32;
            self.ir[0] = self.lm_h(depth >> 12);
        }
    }

    fn dpc(&mut self, use_fifo: bool) {
        let lm = self.lm;

        let r = match use_fifo {
            false => self.rgb.r as i64,
            true => self.rgb_fifo[0].r as i64,
        } << 16;

        let g = match use_fifo {
            false => self.rgb.g as i64,
            true => self.rgb_fifo[0].g as i64,
        } << 16;

        let b = match use_fifo {
            false => self.rgb.b as i64,
            true => self.rgb_fifo[0].b as i64,
        } << 16;

        let rfc = (self.fc.x as i64) << 12;
        let gfc = (self.fc.y as i64) << 12;
        let bfc = (self.fc.z as i64) << 12;

        self.mac[1] = (self.a(1, rfc - r) >> self.sf) as i32;
        self.mac[2] = (self.a(2, gfc - g) >> self.sf) as i32;
        self.mac[3] = (self.a(3, bfc - b) >> self.sf) as i32;

        let mac1 = self.mac[1];
        let mac2 = self.mac[2];
        let mac3 = self.mac[3];

        self.ir[1] = self.lm_b(1, mac1, false);
        self.ir[2] = self.lm_b(2, mac2, false);
        self.ir[3] = self.lm_b(3, mac3, false);

        let ir0 = self.ir[0] as i64;
        let ir1 = self.ir[1] as i64;
        let ir2 = self.ir[2] as i64;
        let ir3 = self.ir[3] as i64;

        self.mac[1] = (self.a(1, r + ir1 * ir0) >> self.sf) as i32;
        self.mac[2] = (self.a(2, g + ir2 * ir0) >> self.sf) as i32;
        self.mac[3] = (self.a(3, b + ir3 * ir0) >> self.sf) as i32;

        let mac1 = self.mac[1];
        let mac2 = self.mac[2];
        let mac3 = self.mac[3];

        self.ir[1] = self.lm_b(1, mac1, lm);
        self.ir[2] = self.lm_b(2, mac2, lm);
        self.ir[3] = self.lm_b(3, mac3, lm);

        let r = self.lm_c(1, mac1 >> 4);
        let g = self.lm_c(2, mac2 >> 4);
        let b = self.lm_c(3, mac3 >> 4);
        let c = self.rgb.c;
        self.push_rgb(r, g, b, c);
    }

    fn nc(&mut self, index: usize) {
        let lm = self.lm;

        let l11 = self.light.m11 as i64;
        let l12 = self.light.m12 as i64;
        let l13 = self.light.m13 as i64;
        let l21 = self.light.m21 as i64;
        let l22 = self.light.m22 as i64;
        let l23 = self.light.m23 as i64;
        let l31 = self.light.m31 as i64;
        let l32 = self.light.m32 as i64;
        let l33 = self.light.m33 as i64;

        let vx = self.v[index].x as i64;
        let vy = self.v[index].y as i64;
        let vz = self.v[index].z as i64;

        let mut temp = [0; 3];

        temp[0] = self.a(1, l11 * vx);
        temp[1] = self.a(2, l21 * vx);
        temp[2] = self.a(3, l31 * vx);

        temp[0] = self.a(1, temp[0] + l12 * vy);
        temp[1] = self.a(2, temp[1] + l22 * vy);
        temp[2] = self.a(3, temp[2] + l32 * vy);

        temp[0] = self.a(1, temp[0] + l13 * vz);
        temp[1] = self.a(2, temp[1] + l23 * vz);
        temp[2] = self.a(3, temp[2] + l33 * vz);

        self.mac[1] = (temp[0] >> self.sf) as i32;
        self.mac[2] = (temp[1] >> self.sf) as i32;
        self.mac[3] = (temp[2] >> self.sf) as i32;

        let mac1 = self.mac[1];
        let mac2 = self.mac[2];
        let mac3 = self.mac[3];

        self.ir[1] = self.lm_b(1, mac1, lm);
        self.ir[2] = self.lm_b(2, mac2, lm);
        self.ir[3] = self.lm_b(3, mac3, lm);

        let rbk = (self.bk.x as i64) << 12;
        let gbk = (self.bk.y as i64) << 12;
        let bbk = (self.bk.z as i64) << 12;

        let ir1 = self.ir[1] as i64;
        let ir2 = self.ir[2] as i64;
        let ir3 = self.ir[3] as i64;

        let c11 = self.colour.m11 as i64;
        let c12 = self.colour.m12 as i64;
        let c13 = self.colour.m13 as i64;
        let c21 = self.colour.m21 as i64;
        let c22 = self.colour.m22 as i64;
        let c23 = self.colour.m23 as i64;
        let c31 = self.colour.m31 as i64;
        let c32 = self.colour.m32 as i64;
        let c33 = self.colour.m33 as i64;

        let mut temp = [0; 3];

        temp[0] = self.a(1, rbk + c11 * ir1);
        temp[1] = self.a(2, gbk + c21 * ir1);
        temp[2] = self.a(3, bbk + c31 * ir1);

        temp[0] = self.a(1, temp[0] + c12 * ir2);
        temp[1] = self.a(2, temp[1] + c22 * ir2);
        temp[2] = self.a(3, temp[2] + c32 * ir2);

        temp[0] = self.a(1, temp[0] + c13 * ir3);
        temp[1] = self.a(2, temp[1] + c23 * ir3);
        temp[2] = self.a(3, temp[2] + c33 * ir3);

        self.mac[1] = (temp[0] >> self.sf) as i32;
        self.mac[2] = (temp[1] >> self.sf) as i32;
        self.mac[3] = (temp[2] >> self.sf) as i32;

        let mac1 = self.mac[1];
        let mac2 = self.mac[2];
        let mac3 = self.mac[3];

        self.ir[1] = self.lm_b(1, mac1, lm);
        self.ir[2] = self.lm_b(2, mac2, lm);
        self.ir[3] = self.lm_b(3, mac3, lm);

        let r = self.lm_c(1, mac1 >> 4);
        let g = self.lm_c(2, mac2 >> 4);
        let b = self.lm_c(3, mac3 >> 4);
        let c = self.rgb.c;
        self.push_rgb(r, g, b, c);
    }

    fn ncc(&mut self, index: usize) {
        let lm = self.lm;

        let l11 = self.light.m11 as i64;
        let l12 = self.light.m12 as i64;
        let l13 = self.light.m13 as i64;
        let l21 = self.light.m21 as i64;
        let l22 = self.light.m22 as i64;
        let l23 = self.light.m23 as i64;
        let l31 = self.light.m31 as i64;
        let l32 = self.light.m32 as i64;
        let l33 = self.light.m33 as i64;

        let vx = self.v[index].x as i64;
        let vy = self.v[index].y as i64;
        let vz = self.v[index].z as i64;

        let mut temp = [0; 3];

        temp[0] = self.a(1, l11 * vx);
        temp[1] = self.a(2, l21 * vx);
        temp[2] = self.a(3, l31 * vx);

        temp[0] = self.a(1, temp[0] + l12 * vy);
        temp[1] = self.a(2, temp[1] + l22 * vy);
        temp[2] = self.a(3, temp[2] + l32 * vy);

        temp[0] = self.a(1, temp[0] + l13 * vz);
        temp[1] = self.a(2, temp[1] + l23 * vz);
        temp[2] = self.a(3, temp[2] + l33 * vz);

        self.mac[1] = (temp[0] >> self.sf) as i32;
        self.mac[2] = (temp[1] >> self.sf) as i32;
        self.mac[3] = (temp[2] >> self.sf) as i32;

        let mac1 = self.mac[1];
        let mac2 = self.mac[2];
        let mac3 = self.mac[3];

        self.ir[1] = self.lm_b(1, mac1, lm);
        self.ir[2] = self.lm_b(2, mac2, lm);
        self.ir[3] = self.lm_b(3, mac3, lm);

        let rbk = (self.bk.x as i64) << 12;
        let gbk = (self.bk.y as i64) << 12;
        let bbk = (self.bk.z as i64) << 12;

        let ir1 = self.ir[1] as i64;
        let ir2 = self.ir[2] as i64;
        let ir3 = self.ir[3] as i64;

        let c11 = self.colour.m11 as i64;
        let c12 = self.colour.m12 as i64;
        let c13 = self.colour.m13 as i64;
        let c21 = self.colour.m21 as i64;
        let c22 = self.colour.m22 as i64;
        let c23 = self.colour.m23 as i64;
        let c31 = self.colour.m31 as i64;
        let c32 = self.colour.m32 as i64;
        let c33 = self.colour.m33 as i64;

        let mut temp = [0; 3];

        temp[0] = self.a(1, rbk + c11 * ir1);
        temp[1] = self.a(2, gbk + c21 * ir1);
        temp[2] = self.a(3, bbk + c31 * ir1);

        temp[0] = self.a(1, temp[0] + c12 * ir2);
        temp[1] = self.a(2, temp[1] + c22 * ir2);
        temp[2] = self.a(3, temp[2] + c32 * ir2);

        temp[0] = self.a(1, temp[0] + c13 * ir3);
        temp[1] = self.a(2, temp[1] + c23 * ir3);
        temp[2] = self.a(3, temp[2] + c33 * ir3);

        self.mac[1] = (temp[0] >> self.sf) as i32;
        self.mac[2] = (temp[1] >> self.sf) as i32;
        self.mac[3] = (temp[2] >> self.sf) as i32;

        let mac1 = self.mac[1];
        let mac2 = self.mac[2];
        let mac3 = self.mac[3];

        self.ir[1] = self.lm_b(1, mac1, lm);
        self.ir[2] = self.lm_b(2, mac2, lm);
        self.ir[3] = self.lm_b(3, mac3, lm);

        let r = (self.rgb.r as i64) << 4;
        let g = (self.rgb.g as i64) << 4;
        let b = (self.rgb.b as i64) << 4;

        let ir1 = self.ir[1] as i64;
        let ir2 = self.ir[2] as i64;
        let ir3 = self.ir[3] as i64;

        self.mac[1] = (self.a(1, r * ir1) >> self.sf) as i32;
        self.mac[2] = (self.a(2, g * ir2) >> self.sf) as i32;
        self.mac[3] = (self.a(3, b * ir3) >> self.sf) as i32;

        let mac1 = self.mac[1];
        let mac2 = self.mac[2];
        let mac3 = self.mac[3];

        self.ir[1] = self.lm_b(1, mac1, lm);
        self.ir[2] = self.lm_b(2, mac2, lm);
        self.ir[3] = self.lm_b(3, mac3, lm);

        let r = self.lm_c(1, mac1 >> 4);
        let g = self.lm_c(2, mac2 >> 4);
        let b = self.lm_c(3, mac3 >> 4);
        let c = self.rgb.c;
        self.push_rgb(r, g, b, c);
    }

    fn ncd(&mut self, index: usize) {
        let lm = self.lm;

        let l11 = self.light.m11 as i64;
        let l12 = self.light.m12 as i64;
        let l13 = self.light.m13 as i64;
        let l21 = self.light.m21 as i64;
        let l22 = self.light.m22 as i64;
        let l23 = self.light.m23 as i64;
        let l31 = self.light.m31 as i64;
        let l32 = self.light.m32 as i64;
        let l33 = self.light.m33 as i64;

        let vx = self.v[index].x as i64;
        let vy = self.v[index].y as i64;
        let vz = self.v[index].z as i64;

        let mut temp = [0; 3];

        temp[0] = self.a(1, l11 * vx);
        temp[1] = self.a(2, l21 * vx);
        temp[2] = self.a(3, l31 * vx);

        temp[0] = self.a(1, temp[0] + l12 * vy);
        temp[1] = self.a(2, temp[1] + l22 * vy);
        temp[2] = self.a(3, temp[2] + l32 * vy);

        temp[0] = self.a(1, temp[0] + l13 * vz);
        temp[1] = self.a(2, temp[1] + l23 * vz);
        temp[2] = self.a(3, temp[2] + l33 * vz);

        self.mac[1] = (temp[0] >> self.sf) as i32;
        self.mac[2] = (temp[1] >> self.sf) as i32;
        self.mac[3] = (temp[2] >> self.sf) as i32;

        let mac1 = self.mac[1];
        let mac2 = self.mac[2];
        let mac3 = self.mac[3];

        self.ir[1] = self.lm_b(1, mac1, lm);
        self.ir[2] = self.lm_b(2, mac2, lm);
        self.ir[3] = self.lm_b(3, mac3, lm);

        let rbk = (self.bk.x as i64) << 12;
        let gbk = (self.bk.y as i64) << 12;
        let bbk = (self.bk.z as i64) << 12;

        let ir1 = self.ir[1] as i64;
        let ir2 = self.ir[2] as i64;
        let ir3 = self.ir[3] as i64;

        let c11 = self.colour.m11 as i64;
        let c12 = self.colour.m12 as i64;
        let c13 = self.colour.m13 as i64;
        let c21 = self.colour.m21 as i64;
        let c22 = self.colour.m22 as i64;
        let c23 = self.colour.m23 as i64;
        let c31 = self.colour.m31 as i64;
        let c32 = self.colour.m32 as i64;
        let c33 = self.colour.m33 as i64;

        let mut temp = [0; 3];

        temp[0] = self.a(1, rbk + c11 * ir1);
        temp[1] = self.a(2, gbk + c21 * ir1);
        temp[2] = self.a(3, bbk + c31 * ir1);

        temp[0] = self.a(1, temp[0] + c12 * ir2);
        temp[1] = self.a(2, temp[1] + c22 * ir2);
        temp[2] = self.a(3, temp[2] + c32 * ir2);

        temp[0] = self.a(1, temp[0] + c13 * ir3);
        temp[1] = self.a(2, temp[1] + c23 * ir3);
        temp[2] = self.a(3, temp[2] + c33 * ir3);

        self.mac[1] = (temp[0] >> self.sf) as i32;
        self.mac[2] = (temp[1] >> self.sf) as i32;
        self.mac[3] = (temp[2] >> self.sf) as i32;

        let mac1 = self.mac[1];
        let mac2 = self.mac[2];
        let mac3 = self.mac[3];

        self.ir[1] = self.lm_b(1, mac1, lm);
        self.ir[2] = self.lm_b(2, mac2, lm);
        self.ir[3] = self.lm_b(3, mac3, lm);

        let prev_ir1 = self.ir[1] as i64;
        let prev_ir2 = self.ir[2] as i64;
        let prev_ir3 = self.ir[3] as i64;

        let r = (self.rgb.r as i64) << 4;
        let g = (self.rgb.g as i64) << 4;
        let b = (self.rgb.b as i64) << 4;

        let rfc = (self.fc.x as i64) << 12;
        let gfc = (self.fc.y as i64) << 12;
        let bfc = (self.fc.z as i64) << 12;

        self.mac[1] = (self.a(1, rfc - r * prev_ir1) >> self.sf) as i32;
        self.mac[2] = (self.a(2, gfc - g * prev_ir2) >> self.sf) as i32;
        self.mac[3] = (self.a(3, bfc - b * prev_ir3) >> self.sf) as i32;

        let mac1 = self.mac[1];
        let mac2 = self.mac[2];
        let mac3 = self.mac[3];

        self.ir[1] = self.lm_b(1, mac1, false);
        self.ir[2] = self.lm_b(2, mac2, false);
        self.ir[3] = self.lm_b(3, mac3, false);

        let ir0 = self.ir[0] as i64;
        let ir1 = self.ir[1] as i64;
        let ir2 = self.ir[2] as i64;
        let ir3 = self.ir[3] as i64;

        self.mac[1] = (self.a(1, (r * prev_ir1) + ir0 * ir1) >> self.sf) as i32;
        self.mac[2] = (self.a(2, (g * prev_ir2) + ir0 * ir2) >> self.sf) as i32;
        self.mac[3] = (self.a(3, (b * prev_ir3) + ir0 * ir3) >> self.sf) as i32;

        let mac1 = self.mac[1];
        let mac2 = self.mac[2];
        let mac3 = self.mac[3];

        self.ir[1] = self.lm_b(1, mac1, lm);
        self.ir[2] = self.lm_b(2, mac2, lm);
        self.ir[3] = self.lm_b(3, mac3, lm);

        let r = self.lm_c(1, mac1 >> 4);
        let g = self.lm_c(2, mac2 >> 4);
        let b = self.lm_c(3, mac3 >> 4);
        let c = self.rgb.c;
        self.push_rgb(r, g, b, c);
    }

    fn saturate_i64_to_i44(value: i64) -> i64 {
        (value << 20) >> 20
    }

    fn saturate_i16_to_u5(value: i16) -> u8 {
        if value > 0x1f {
            return 0x1f;
        }

        if value < 0 {
            return 0;
        }

        value as u8
    }

    fn a(&mut self, index: usize, value: i64) -> i64 {
        if value < -0x800_0000_0000 {
            self.flags |= 0x800_0000 >> (index - 1);
        }

        if value > 0x7ff_ffff_ffff {
            self.flags |= 0x4000_0000 >> (index - 1);
        }

        Gte::saturate_i64_to_i44(value)
    }

    fn lm_b(&mut self, index: usize, value: i32, lm: bool) -> i16 {
        if lm && value < 0 {
            self.flags |= 0x100_0000 >> (index - 1);
            return 0;
        }

        if !lm && value < -0x8000 {
            self.flags |= 0x100_0000 >> (index - 1);
            return -0x8000;
        }

        if value > 0x7fff {
            self.flags |= 0x100_0000 >> (index - 1);
            return 0x7fff;
        }

        value as i16
    }

    fn lm_b_z(&mut self, value: i32, old: i64, lm: bool) -> i16 {
        if old < -0x8000 {
            self.flags |= 0x40_0000;
        }

        if old > 0x7fff {
            self.flags |= 0x40_0000;
        }

        if lm && value < 0 {
            return 0;
        }

        if !lm && value < -0x8000 {
            return -0x8000;
        }

        if value > 0x7fff {
            return 0x7fff;
        }

        value as i16
    }

    fn lm_c(&mut self, index: usize, value: i32) -> u8 {
        if value < 0 {
            self.flags |= 0x20_0000 >> (index - 1);
            return 0;
        }

        if value > 0xff {
            self.flags |= 0x20_0000 >> (index - 1);
            return 0xff;
        }

        value as u8
    }

    fn lm_d(&mut self, value: i64) -> u16 {
        if value < 0 {
            self.flags |= 0x4_0000;
            return 0;
        }

        if value > 0xffff {
            self.flags |= 0x4_0000;
            return 0xffff;
        }

        value as u16
    }

    fn f(&mut self, value: i64) -> i64 {
        if value < -0x8000_0000 {
            self.flags |= 0x8000;
        } else if value > 0x7fff_ffff {
            self.flags |= 0x1_0000;
        }

        value
    }

    fn lm_g(&mut self, index: usize, value: i32) -> i16 {
        if value < -0x400 {
            self.flags |= 0x4000 >> (index - 1);
            return -0x400;
        }

        if value > 0x3ff {
            self.flags |= 0x4000 >> (index - 1);
            return 0x3ff;
        }

        value as i16
    }

    fn lm_h(&mut self, value: i64) -> i16 {
        if value < 0 {
            self.flags |= 0x1000;
            return 0;
        }

        if value > 0x1000 {
            self.flags |= 0x1000;
            return 0x1000;
        }

        value as i16
    }

    pub fn divide(numerator: u16, divisor: u16) -> u32 {
        let z = divisor.leading_zeros();
        let n = (numerator as u64) << z;
        let d = (divisor as u64) << z;
        let u = UNR_TABLE[(d as usize - 0x7fc0) >> 7] as u64 + 0x101;
        let d2 = (0x2000080 - (d * u)) >> 8;
        let d3 = (0x80 + (d2 * u)) >> 8;
        cmp::min(0x1ffff, (((n*d3) + 0x8000) >> 16) as u32)
    }

    fn push_sx(&mut self, sx: i16) {
        self.sxy_fifo[0].x = self.sxy_fifo[1].x;
        self.sxy_fifo[1].x = self.sxy_fifo[2].x;
        self.sxy_fifo[2].x = sx;
    }

    fn push_sy(&mut self, sy: i16) {
        self.sxy_fifo[0].y = self.sxy_fifo[1].y;
        self.sxy_fifo[1].y = self.sxy_fifo[2].y;
        self.sxy_fifo[2].y = sy;
    }

    fn push_sz(&mut self, sz: u16) {
        self.sz_fifo[0] = self.sz_fifo[1];
        self.sz_fifo[1] = self.sz_fifo[2];
        self.sz_fifo[2] = self.sz_fifo[3];
        self.sz_fifo[3] = sz;
    }

    fn push_rgb(&mut self, r: u8, g: u8, b: u8, c: u8) {
        self.rgb_fifo[0] = self.rgb_fifo[1];
        self.rgb_fifo[1] = self.rgb_fifo[2];

        self.rgb_fifo[2].r = r;
        self.rgb_fifo[2].g = g;
        self.rgb_fifo[2].b = b;
        self.rgb_fifo[2].c = c;
    }

    pub fn read_control(&self, index: usize) -> u32 {
        match index {
            0 => ((self.rotation.m12 as u16 as u32) << 16) | (self.rotation.m11 as u16 as u32),
            1 => ((self.rotation.m21 as u16 as u32) << 16) | (self.rotation.m13 as u16 as u32),
            2 => ((self.rotation.m23 as u16 as u32) << 16) | (self.rotation.m22 as u16 as u32),
            3 => ((self.rotation.m32 as u16 as u32) << 16) | (self.rotation.m31 as u16 as u32),
            4 => self.rotation.m33 as u32,
            5 => self.tr.x as u32,
            6 => self.tr.y as u32,
            7 => self.tr.z as u32,
            8 => ((self.light.m12 as u16 as u32) << 16) | (self.light.m11 as u16 as u32),
            9 => ((self.light.m21 as u16 as u32) << 16) | (self.light.m13 as u16 as u32),
            10 => ((self.light.m23 as u16 as u32) << 16) | (self.light.m22 as u16 as u32),
            11 => ((self.light.m32 as u16 as u32) << 16) | (self.light.m31 as u16 as u32),
            12 => self.light.m33 as u32,
            13 => self.bk.x as u32,
            14 => self.bk.y as u32,
            15 => self.bk.z as u32,
            16 => ((self.colour.m12 as u16 as u32) << 16) | (self.colour.m11 as u16 as u32),
            17 => ((self.colour.m21 as u16 as u32) << 16) | (self.colour.m13 as u16 as u32),
            18 => ((self.colour.m23 as u16 as u32) << 16) | (self.colour.m22 as u16 as u32),
            19 => ((self.colour.m32 as u16 as u32) << 16) | (self.colour.m31 as u16 as u32),
            20 => self.colour.m33 as u32,
            21 => self.fc.x as u32,
            22 => self.fc.y as u32,
            23 => self.fc.z as u32,
            24 => self.ofx as u32,
            25 => self.ofy as u32,
            26 => self.h as i16 as u32,
            27 => self.dqa as u32,
            28 => self.dqb as u32,
            29 => self.zsf3 as u32,
            30 => self.zsf4 as u32,
            31 => self.flags,
            _ => panic!(
                "[GTE] [ERROR] Read from unknown control register cop2r{}",
                index
            ),
        }
    }

    pub fn write_control(&mut self, index: usize, value: u32) {
        match index {
            0 => {
                self.rotation.m11 = value as i16;
                self.rotation.m12 = (value >> 16) as i16;
            }
            1 => {
                self.rotation.m13 = value as i16;
                self.rotation.m21 = (value >> 16) as i16;
            }
            2 => {
                self.rotation.m22 = value as i16;
                self.rotation.m23 = (value >> 16) as i16;
            }
            3 => {
                self.rotation.m31 = value as i16;
                self.rotation.m32 = (value >> 16) as i16;
            }
            4 => self.rotation.m33 = value as i16,
            5 => self.tr.x = value as i32,
            6 => self.tr.y = value as i32,
            7 => self.tr.z = value as i32,
            8 => {
                self.light.m11 = value as i16;
                self.light.m12 = (value >> 16) as i16;
            }
            9 => {
                self.light.m13 = value as i16;
                self.light.m21 = (value >> 16) as i16;
            }
            10 => {
                self.light.m22 = value as i16;
                self.light.m23 = (value >> 16) as i16;
            }
            11 => {
                self.light.m31 = value as i16;
                self.light.m32 = (value >> 16) as i16;
            }
            12 => self.light.m33 = value as i16,
            13 => self.bk.x = value as i32,
            14 => self.bk.y = value as i32,
            15 => self.bk.z = value as i32,
            16 => {
                self.colour.m11 = value as i16;
                self.colour.m12 = (value >> 16) as i16;
            }
            17 => {
                self.colour.m13 = value as i16;
                self.colour.m21 = (value >> 16) as i16;
            }
            18 => {
                self.colour.m22 = value as i16;
                self.colour.m23 = (value >> 16) as i16;
            }
            19 => {
                self.colour.m31 = value as i16;
                self.colour.m32 = (value >> 16) as i16;
            }
            20 => self.colour.m33 = value as i16,
            21 => self.fc.x = value as i32,
            22 => self.fc.y = value as i32,
            23 => self.fc.z = value as i32,
            24 => self.ofx = value as i32,
            25 => self.ofy = value as i32,
            26 => self.h = value as u16,
            27 => self.dqa = value as i16,
            28 => self.dqb = value as i32,
            29 => self.zsf3 = value as i16,
            30 => self.zsf4 = value as i16,
            31 => {
                self.flags = value & 0x7fff_f000;

                if (value & 0x7f87e000) != 0 {
                    self.flags |= 0x8000_0000;
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn read_data(&self, index: usize) -> u32 {
        match index {
            0 => (self.v[0].x as u16 as u32) | ((self.v[0].y as u16 as u32) << 16),
            1 => self.v[0].z as u32,
            2 => (self.v[1].x as u16 as u32) | ((self.v[1].y as u16 as u32) << 16),
            3 => self.v[1].z as u32,
            4 => (self.v[2].x as u16 as u32) | ((self.v[2].y as u16 as u32) << 16),
            5 => self.v[2].z as u32,
            6 => {
                let r = self.rgb.r as u32;
                let g = self.rgb.g as u32;
                let b = self.rgb.b as u32;
                let c = self.rgb.c as u32;

                r | (g << 8) | (b << 16) | (c << 24)
            }
            7 => self.otz as u32,
            8 => self.ir[0] as i32 as u32,
            9 => self.ir[1] as i32 as u32,
            10 => self.ir[2] as i32 as u32,
            11 => self.ir[3] as i32 as u32,
            12 => (self.sxy_fifo[0].x as u16 as u32) | ((self.sxy_fifo[0].y as u16 as u32) << 16),
            13 => (self.sxy_fifo[1].x as u16 as u32) | ((self.sxy_fifo[1].y as u16 as u32) << 16),
            14 | 15 => {
                (self.sxy_fifo[2].x as u16 as u32) | ((self.sxy_fifo[2].y as u16 as u32) << 16)
            }
            16 => self.sz_fifo[0] as u32,
            17 => self.sz_fifo[1] as u32,
            18 => self.sz_fifo[2] as u32,
            19 => self.sz_fifo[3] as u32,
            20 | 21 | 22 => {
                let mut value = 0;

                value |= (self.rgb_fifo[index - 20].c as u32) << 24;
                value |= (self.rgb_fifo[index - 20].b as u32) << 16;
                value |= (self.rgb_fifo[index - 20].g as u32) << 8;
                value |= self.rgb_fifo[index - 20].r as u32;

                value
            }
            23 => self.res1,
            24 => self.mac[0] as u32,
            25 => self.mac[1] as u32,
            26 => self.mac[2] as u32,
            27 => self.mac[3] as u32,
            28 | 29 => {
                let r = Gte::saturate_i16_to_u5(self.ir[1] >> 7) as u32;
                let g = Gte::saturate_i16_to_u5(self.ir[2] >> 7) as u32;
                let b = Gte::saturate_i16_to_u5(self.ir[3] >> 7) as u32;

                r | (g << 5) | (b << 10)
            }
            30 => self.lzcs as u32,
            31 => self.lzcr as u32,
            _ => panic!(
                "[GTE] [ERROR] Read from unknown data register cop2r{}",
                index
            ),
        }
    }

    pub fn leading_count(lzcs: i32) -> u32 {
        let leading_bit = (lzcs as u32) >> 31;
        let mut leading_count = 1;

        for i in 1..32 {
            if (((lzcs as u32) >> (31 - i)) & 0x1) == leading_bit {
                leading_count += 1;
            } else {
                break;
            }
        }

        leading_count
    }

    pub fn write_data(&mut self, index: usize, value: u32) {
        match index {
            0 => {
                self.v[0].x = value as i16;
                self.v[0].y = (value >> 16) as i16;
            }
            1 => self.v[0].z = value as i16,
            2 => {
                self.v[1].x = value as i16;
                self.v[1].y = (value >> 16) as i16;
            }
            3 => self.v[1].z = value as i16,
            4 => {
                self.v[2].x = value as i16;
                self.v[2].y = (value >> 16) as i16;
            }
            5 => self.v[2].z = value as i16,
            6 => {
                self.rgb.r = value as u8;
                self.rgb.g = (value >> 8) as u8;
                self.rgb.b = (value >> 16) as u8;
                self.rgb.c = (value >> 24) as u8;
            }
            7 => self.otz = value as u16,
            8 => self.ir[0] = value as i16,
            9 => self.ir[1] = value as i16,
            10 => self.ir[2] = value as i16,
            11 => self.ir[3] = value as i16,
            12 => {
                self.sxy_fifo[0].x = value as i16;
                self.sxy_fifo[0].y = (value >> 16) as i16;
            }
            13 => {
                self.sxy_fifo[1].x = value as i16;
                self.sxy_fifo[1].y = (value >> 16) as i16;
            }
            14 => {
                self.sxy_fifo[2].x = value as i16;
                self.sxy_fifo[2].y = (value >> 16) as i16;
            }
            15 => {
                self.push_sx(value as i16);
                self.push_sy((value >> 16) as i16);
            }
            16 => self.sz_fifo[0] = value as u16,
            17 => self.sz_fifo[1] = value as u16,
            18 => self.sz_fifo[2] = value as u16,
            19 => self.sz_fifo[3] = value as u16,
            20 | 21 | 22 => {
                self.rgb_fifo[index - 20].r = value as u8;
                self.rgb_fifo[index - 20].g = (value >> 8) as u8;
                self.rgb_fifo[index - 20].b = (value >> 16) as u8;
                self.rgb_fifo[index - 20].c = (value >> 24) as u8;
            }
            23 => self.res1 = value,
            24 => self.mac[0] = value as i32,
            25 => self.mac[1] = value as i32,
            26 => self.mac[2] = value as i32,
            27 => self.mac[3] = value as i32,
            28 => {
                self.ir[1] = ((value & 0x1f) << 7) as i16;
                self.ir[2] = (((value >> 5) & 0x1f) << 7) as i16;
                self.ir[3] = (((value >> 10) & 0x1f) << 7) as i16;
            }
            29 => (),
            30 => {
                self.lzcs = value as i32;
                self.lzcr = Gte::leading_count(self.lzcs) as i32;
            }
            31 => (),
            _ => panic!(
                "[GTE] [ERROR] Write to unknown data register cop2r{}",
                index
            ),
        }
    }
}
