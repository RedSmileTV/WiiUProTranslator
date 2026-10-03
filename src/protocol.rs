//! Wii U Pro Controller protocol and XInput mapping.

pub const NINTENDO_VID: u16 = 0x057E;
pub const WIIU_PRO_PID: u16 = 0x0330;

/// Output report size: Report ID (1 byte, 0x00) + payload (22 bytes).
pub const OUTPUT_REPORT_LEN: usize = 23;

pub const IN_STATUS: u8 = 0x20;
pub const IN_EXT_21: u8 = 0x3D;

const OUT_LEDS: u8 = 0x11;
const OUT_REPORT_MODE: u8 = 0x12;
const OUT_STATUS_REQUEST: u8 = 0x15;
const OUT_WRITE_MEMORY: u8 = 0x16;

pub type OutputReport = [u8; OUTPUT_REPORT_LEN];

pub fn led_report(mask: u8) -> OutputReport {
    let mut r = [0u8; OUTPUT_REPORT_LEN];
    r[0] = 0x00; // Windows HID Report ID
    r[1] = OUT_LEDS;
    r[2] = (mask & 0x0F) << 4;
    r
}

pub fn report_mode_report(mode: u8) -> OutputReport {
    let mut r = [0u8; OUTPUT_REPORT_LEN];
    r[0] = 0x00;
    r[1] = OUT_REPORT_MODE;
    r[2] = 0x04; // continuous reporting
    r[3] = mode;
    r
}

pub fn status_request_report() -> OutputReport {
    let mut r = [0u8; OUTPUT_REPORT_LEN];
    r[0] = 0x00;
    r[1] = OUT_STATUS_REQUEST;
    r
}

pub fn write_register_report(addr: u32, data: &[u8]) -> OutputReport {
    assert!(data.len() <= 16, "register writes are at most 16 bytes");
    let a = addr.to_be_bytes();
    let mut r = [0u8; OUTPUT_REPORT_LEN];
    r[0] = 0x00;
    r[1] = OUT_WRITE_MEMORY;
    r[2] = 0x04; // control register address space
    r[3] = a[1];
    r[4] = a[2];
    r[5] = a[3];
    r[6] = data.len() as u8;
    r[7..7 + data.len()].copy_from_slice(data);
    r
}

pub fn init_sequence() -> Vec<OutputReport> {
    vec![
        write_register_report(0x00A4_00F0, &[0x55]),
        write_register_report(0x00A4_00FB, &[0x00]),
        led_report(0b0001),
        report_mode_report(IN_EXT_21),
    ]
}

pub mod pro {
    pub const DPAD_UP: u16 = 1 << 0;
    pub const DPAD_DOWN: u16 = 1 << 1;
    pub const DPAD_LEFT: u16 = 1 << 2;
    pub const DPAD_RIGHT: u16 = 1 << 3;
    pub const A: u16 = 1 << 4;
    pub const B: u16 = 1 << 5;
    pub const X: u16 = 1 << 6;
    pub const Y: u16 = 1 << 7;
    pub const L: u16 = 1 << 8;
    pub const R: u16 = 1 << 9;
    pub const ZL: u16 = 1 << 10;
    pub const ZR: u16 = 1 << 11;
    pub const PLUS: u16 = 1 << 12;
    pub const MINUS: u16 = 1 << 13;
    pub const HOME: u16 = 1 << 14;
    pub const L3: u16 = 1 << 15;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProState {
    pub buttons: u16,
    pub r3: bool,
    pub lx: u16,
    pub ly: u16,
    pub rx: u16,
    pub ry: u16,
}

pub fn parse_input_report(buf: &[u8]) -> Option<ProState> {
    if buf.len() < 12 {
        return None;
    }

    // Determine report ID offset depending on whether hidapi prefixed a Report ID byte
    let (report_id, payload) = if buf[0] == IN_EXT_21 {
        (buf[0], &buf[1..])
    } else if buf.len() >= 13 && buf[1] == IN_EXT_21 {
        (buf[1], &buf[2..])
    } else {
        return None;
    };

    if report_id != IN_EXT_21 || payload.len() < 11 {
        return None;
    }

    let e = &payload[..11];
    let stick = |lo: u8, hi: u8| u16::from(lo) | (u16::from(hi & 0x0F) << 8);

    let b8 = !e[8];
    let b9 = !e[9];
    let b10 = !e[10];

    let table: [(bool, u16); 16] = [
        (b9 & 0x01 != 0, pro::DPAD_UP),
        (b8 & 0x40 != 0, pro::DPAD_DOWN),
        (b9 & 0x02 != 0, pro::DPAD_LEFT),
        (b8 & 0x80 != 0, pro::DPAD_RIGHT),
        (b9 & 0x10 != 0, pro::A),
        (b9 & 0x40 != 0, pro::B),
        (b9 & 0x08 != 0, pro::X),
        (b9 & 0x20 != 0, pro::Y),
        (b8 & 0x20 != 0, pro::L),
        (b8 & 0x02 != 0, pro::R),
        (b9 & 0x80 != 0, pro::ZL),
        (b9 & 0x04 != 0, pro::ZR),
        (b8 & 0x04 != 0, pro::PLUS),
        (b8 & 0x10 != 0, pro::MINUS),
        (b8 & 0x08 != 0, pro::HOME),
        (b10 & 0x02 != 0, pro::L3),
    ];

    let buttons = table
        .iter()
        .filter(|(down, _)| *down)
        .fold(0u16, |acc, (_, bit)| acc | bit);

    Some(ProState {
        buttons,
        r3: b10 & 0x01 != 0,
        lx: stick(e[0], e[1]),
        rx: stick(e[2], e[3]),
        ly: stick(e[4], e[5]),
        ry: stick(e[6], e[7]),
    })
}

pub mod xb {
    pub const UP: u16 = 0x0001;
    pub const DOWN: u16 = 0x0002;
    pub const LEFT: u16 = 0x0004;
    pub const RIGHT: u16 = 0x0008;
    pub const START: u16 = 0x0010;
    pub const BACK: u16 = 0x0020;
    pub const LTHUMB: u16 = 0x0040;
    pub const RTHUMB: u16 = 0x0080;
    pub const LB: u16 = 0x0100;
    pub const RB: u16 = 0x0200;
    pub const GUIDE: u16 = 0x0400;
    pub const A: u16 = 0x1000;
    pub const B: u16 = 0x2000;
    pub const X: u16 = 0x4000;
    pub const Y: u16 = 0x8000;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct XState {
    pub buttons: u16,
    pub left_trigger: u8,
    pub right_trigger: u8,
    pub lx: i16,
    pub ly: i16,
    pub rx: i16,
    pub ry: i16,
}

pub const STICK_CENTER: f32 = 2048.0;

#[derive(Clone, Copy, Debug)]
pub struct MapConfig {
    pub nintendo_layout: bool,
    pub deadzone: f32,
    pub range: f32,
}

impl Default for MapConfig {
    fn default() -> Self {
        Self { nintendo_layout: false, deadzone: 0.08, range: 1100.0 }
    }
}

impl MapConfig {
    pub fn sanitized(mut self) -> Self {
        self.deadzone = if self.deadzone.is_finite() { self.deadzone.clamp(0.0, 0.9) } else { 0.08 };
        self.range = if self.range.is_finite() { self.range.max(100.0) } else { 1100.0 };
        self
    }
}

fn stick_to_xinput(x_raw: u16, y_raw: u16, cfg: &MapConfig) -> (i16, i16) {
    let nx = (f32::from(x_raw) - STICK_CENTER) / cfg.range;
    let ny = (f32::from(y_raw) - STICK_CENTER) / cfg.range;
    let mag = (nx * nx + ny * ny).sqrt();
    if mag <= cfg.deadzone || mag == 0.0 {
        return (0, 0);
    }
    let scaled = ((mag - cfg.deadzone) / (1.0 - cfg.deadzone)).min(1.0);
    let k = scaled / mag;
    let conv = |v: f32| (v * k * 32767.0).round().clamp(-32767.0, 32767.0) as i16;
    (conv(nx), conv(ny))
}

pub fn map_to_xinput(s: &ProState, cfg: &MapConfig) -> XState {
    let mut b = 0u16;
    let mut map = |down: bool, bit: u16| {
        if down {
            b |= bit;
        }
    };

    map(s.buttons & pro::DPAD_UP != 0, xb::UP);
    map(s.buttons & pro::DPAD_DOWN != 0, xb::DOWN);
    map(s.buttons & pro::DPAD_LEFT != 0, xb::LEFT);
    map(s.buttons & pro::DPAD_RIGHT != 0, xb::RIGHT);

    let (a, bb, x, y) = if cfg.nintendo_layout {
        (xb::A, xb::B, xb::X, xb::Y)
    } else {
        (xb::B, xb::A, xb::Y, xb::X)
    };
    map(s.buttons & pro::A != 0, a);
    map(s.buttons & pro::B != 0, bb);
    map(s.buttons & pro::X != 0, x);
    map(s.buttons & pro::Y != 0, y);

    map(s.buttons & pro::L != 0, xb::LB);
    map(s.buttons & pro::R != 0, xb::RB);
    map(s.buttons & pro::PLUS != 0, xb::START);
    map(s.buttons & pro::MINUS != 0, xb::BACK);
    map(s.buttons & pro::HOME != 0, xb::GUIDE);
    map(s.buttons & pro::L3 != 0, xb::LTHUMB);
    map(s.r3, xb::RTHUMB);

    let (lx, ly) = stick_to_xinput(s.lx, s.ly, cfg);
    let (rx, ry) = stick_to_xinput(s.rx, s.ry, cfg);

    XState {
        buttons: b,
        left_trigger: if s.buttons & pro::ZL != 0 { 255 } else { 0 },
        right_trigger: if s.buttons & pro::ZR != 0 { 255 } else { 0 },
        lx,
        ly,
        rx,
        ry,
    }
}