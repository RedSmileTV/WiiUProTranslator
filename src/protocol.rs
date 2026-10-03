//! Wii U Pro Controller protocol and XInput mapping.
//!
//! The Wii U Pro Controller speaks the Wii Remote HID protocol over Bluetooth.
//! Everything in this file is plain `std` with no platform code, so it can be
//! unit-tested on any OS.

/// Nintendo's Bluetooth vendor id.
pub const NINTENDO_VID: u16 = 0x057E;
/// Product id of the Wii U Pro Controller ("Nintendo RVL-CNT-01-UC").
pub const WIIU_PRO_PID: u16 = 0x0330;

/// Every output report is sent zero-padded to this size (report id + 21 bytes).
/// That is what the controller's HID descriptor declares and what Windows expects.
pub const OUTPUT_REPORT_LEN: usize = 22;

/// Input report: status (battery, LEDs, extension-connected flag).
pub const IN_STATUS: u8 = 0x20;
/// Input report: 21 extension bytes and nothing else. The Pro Controller's
/// buttons and sticks live in the first 11 of them.
pub const IN_EXT_21: u8 = 0x3D;

const OUT_LEDS: u8 = 0x11;
const OUT_REPORT_MODE: u8 = 0x12;
const OUT_STATUS_REQUEST: u8 = 0x15;
const OUT_WRITE_MEMORY: u8 = 0x16;

pub type OutputReport = [u8; OUTPUT_REPORT_LEN];

/// Set the player LEDs (bit 0 = LED 1 ... bit 3 = LED 4).
pub fn led_report(mask: u8) -> OutputReport {
    let mut r = [0u8; OUTPUT_REPORT_LEN];
    r[0] = OUT_LEDS;
    r[1] = (mask & 0x0F) << 4;
    r
}

/// Choose the data reporting mode, with continuous reporting enabled.
pub fn report_mode_report(mode: u8) -> OutputReport {
    let mut r = [0u8; OUTPUT_REPORT_LEN];
    r[0] = OUT_REPORT_MODE;
    r[1] = 0x04; // continuous reporting
    r[2] = mode;
    r
}

/// Ask the controller to send a status report (`IN_STATUS`).
pub fn status_request_report() -> OutputReport {
    let mut r = [0u8; OUTPUT_REPORT_LEN];
    r[0] = OUT_STATUS_REQUEST;
    r
}

/// Write up to 16 bytes into the controller's control-register space.
/// `addr` is the 24-bit register address, e.g. `0xA400F0`.
pub fn write_register_report(addr: u32, data: &[u8]) -> OutputReport {
    assert!(data.len() <= 16, "register writes are at most 16 bytes");
    let a = addr.to_be_bytes();
    let mut r = [0u8; OUTPUT_REPORT_LEN];
    r[0] = OUT_WRITE_MEMORY;
    r[1] = 0x04; // address space: control registers
    r[2] = a[1];
    r[3] = a[2];
    r[4] = a[3];
    r[5] = data.len() as u8;
    r[6..6 + data.len()].copy_from_slice(data);
    r
}

/// Reports to send, in order, to get the controller streaming `IN_EXT_21`
/// reports: unencrypted extension handshake, player LED 1, reporting mode.
pub fn init_sequence() -> Vec<OutputReport> {
    vec![
        write_register_report(0x00A4_00F0, &[0x55]),
        write_register_report(0x00A4_00FB, &[0x00]),
        led_report(0b0001),
        report_mode_report(IN_EXT_21),
    ]
}

/// Button bits of [`ProState::buttons`] (1 = pressed). R3 is `ProState::r3`.
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

/// Decoded Pro Controller input. Stick values are the raw 12-bit readings
/// (0..=4095, about 2048 at rest, larger = right / up).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProState {
    pub buttons: u16,
    pub r3: bool,
    pub lx: u16,
    pub ly: u16,
    pub rx: u16,
    pub ry: u16,
}

/// Decode an `IN_EXT_21` input report (report id first). Returns `None` for
/// any other report or a truncated one.
pub fn parse_input_report(buf: &[u8]) -> Option<ProState> {
    if buf.len() < 12 || buf[0] != IN_EXT_21 {
        return None;
    }
    let e = &buf[1..12];
    let stick = |lo: u8, hi: u8| u16::from(lo) | (u16::from(hi & 0x0F) << 8);

    // Button bits are active-low on the wire; invert so 1 = pressed.
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

/// XInput button bits (identical to `XINPUT_GAMEPAD_*` / ViGEm `XButtons`).
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

/// Xbox 360 pad state, ready to hand to the virtual controller.
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

/// Raw stick reading at rest.
pub const STICK_CENTER: f32 = 2048.0;

#[derive(Clone, Copy, Debug)]
pub struct MapConfig {
    /// `false` (default): face buttons keep their *position* (Pro B, the bottom
    /// button, becomes Xbox A). `true`: face buttons keep their *label*
    /// (Pro A becomes Xbox A).
    pub nintendo_layout: bool,
    /// Radial dead zone as a fraction of full deflection, 0.0..0.9.
    pub deadzone: f32,
    /// Raw units from centre to full deflection of a stick.
    pub range: f32,
}

impl Default for MapConfig {
    fn default() -> Self {
        Self { nintendo_layout: false, deadzone: 0.08, range: 1100.0 }
    }
}

impl MapConfig {
    /// Clamp user-supplied values into a usable range.
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
    // Rescale so the output starts at 0 right outside the dead zone and
    // reaches full scale at full deflection, without distorting direction.
    let scaled = ((mag - cfg.deadzone) / (1.0 - cfg.deadzone)).min(1.0);
    let k = scaled / mag;
    let conv = |v: f32| (v * k * 32767.0).round().clamp(-32767.0, 32767.0) as i16;
    (conv(nx), conv(ny))
}

/// Convert a Pro Controller state into an Xbox 360 pad state.
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

    // (Pro A, Pro B, Pro X, Pro Y) -> Xbox button
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
        // ZL / ZR are digital buttons on the Pro Controller.
        left_trigger: if s.buttons & pro::ZL != 0 { 255 } else { 0 },
        right_trigger: if s.buttons & pro::ZR != 0 { 255 } else { 0 },
        lx,
        ly,
        rx,
        ry,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A neutral 0x3D report: sticks centred, no buttons pressed.
    /// Report byte N holds extension byte N-1 (byte 0 is the report id).
    fn neutral() -> Vec<u8> {
        let mut r = vec![0u8; 22];
        r[0] = IN_EXT_21;
        for i in 0..4 {
            r[1 + i * 2] = 0x00;
            r[2 + i * 2] = 0x08; // 0x0800 = 2048
        }
        r[9] = 0xFF;
        r[10] = 0xFF;
        r[11] = 0xFF;
        r
    }

    fn state(report: &[u8]) -> ProState {
        parse_input_report(report).expect("valid report")
    }

    fn idle() -> ProState {
        ProState { lx: 2048, ly: 2048, rx: 2048, ry: 2048, ..Default::default() }
    }

    #[test]
    fn neutral_report_is_idle() {
        let s = state(&neutral());
        assert_eq!(s.buttons, 0);
        assert!(!s.r3);
        assert_eq!((s.lx, s.ly, s.rx, s.ry), (2048, 2048, 2048, 2048));
        assert_eq!(map_to_xinput(&s, &MapConfig::default()), XState::default());
    }

    #[test]
    fn rejects_other_or_short_reports() {
        assert!(parse_input_report(&[]).is_none());
        assert!(parse_input_report(&[IN_STATUS; 22]).is_none());
        assert!(parse_input_report(&neutral()[..11]).is_none());
    }

    #[test]
    fn decodes_every_button() {
        // (report byte index, active-low bit, expected Pro button)
        let table: [(usize, u8, u16); 16] = [
            (10, 0x01, pro::DPAD_UP),
            (9, 0x40, pro::DPAD_DOWN),
            (10, 0x02, pro::DPAD_LEFT),
            (9, 0x80, pro::DPAD_RIGHT),
            (10, 0x10, pro::A),
            (10, 0x40, pro::B),
            (10, 0x08, pro::X),
            (10, 0x20, pro::Y),
            (9, 0x20, pro::L),
            (9, 0x02, pro::R),
            (10, 0x80, pro::ZL),
            (10, 0x04, pro::ZR),
            (9, 0x04, pro::PLUS),
            (9, 0x10, pro::MINUS),
            (9, 0x08, pro::HOME),
            (11, 0x02, pro::L3),
        ];
        for (idx, mask, want) in table {
            let mut r = neutral();
            r[idx] &= !mask;
            let s = state(&r);
            assert_eq!(s.buttons, want, "report byte {idx} mask {mask:#04x}");
            assert!(!s.r3);
        }
        let mut r = neutral();
        r[11] &= !0x01;
        assert!(state(&r).r3);
    }

    #[test]
    fn sticks_use_only_the_low_12_bits() {
        let mut r = neutral();
        r[1] = 0x34;
        r[2] = 0xF2; // upper nibble must be ignored
        r[5] = 0xFF;
        r[6] = 0x0F; // left stick Y = 0x0FFF
        let s = state(&r);
        assert_eq!(s.lx, 0x234);
        assert_eq!(s.ly, 0x0FFF);
    }

    #[test]
    fn face_buttons_follow_position_by_default() {
        let cfg = MapConfig::default();
        let map = |btn| map_to_xinput(&ProState { buttons: btn, ..idle() }, &cfg).buttons;
        assert_eq!(map(pro::B), xb::A); // bottom
        assert_eq!(map(pro::A), xb::B); // right
        assert_eq!(map(pro::Y), xb::X); // left
        assert_eq!(map(pro::X), xb::Y); // top
    }

    #[test]
    fn face_buttons_follow_label_with_nintendo_layout() {
        let cfg = MapConfig { nintendo_layout: true, ..MapConfig::default() };
        let map = |btn| map_to_xinput(&ProState { buttons: btn, ..idle() }, &cfg).buttons;
        assert_eq!(map(pro::A), xb::A);
        assert_eq!(map(pro::B), xb::B);
        assert_eq!(map(pro::X), xb::X);
        assert_eq!(map(pro::Y), xb::Y);
    }

    #[test]
    fn other_buttons_and_triggers() {
        let cfg = MapConfig::default();
        let map = |btn| map_to_xinput(&ProState { buttons: btn, ..idle() }, &cfg);
        assert_eq!(map(pro::DPAD_UP).buttons, xb::UP);
        assert_eq!(map(pro::DPAD_DOWN).buttons, xb::DOWN);
        assert_eq!(map(pro::DPAD_LEFT).buttons, xb::LEFT);
        assert_eq!(map(pro::DPAD_RIGHT).buttons, xb::RIGHT);
        assert_eq!(map(pro::L).buttons, xb::LB);
        assert_eq!(map(pro::R).buttons, xb::RB);
        assert_eq!(map(pro::PLUS).buttons, xb::START);
        assert_eq!(map(pro::MINUS).buttons, xb::BACK);
        assert_eq!(map(pro::HOME).buttons, xb::GUIDE);
        assert_eq!(map(pro::L3).buttons, xb::LTHUMB);
        let r3 = map_to_xinput(&ProState { r3: true, ..idle() }, &cfg);
        assert_eq!(r3.buttons, xb::RTHUMB);
        let zl = map(pro::ZL);
        assert_eq!((zl.left_trigger, zl.right_trigger, zl.buttons), (255, 0, 0));
        let zr = map(pro::ZR);
        assert_eq!((zr.left_trigger, zr.right_trigger, zr.buttons), (0, 255, 0));
    }

    #[test]
    fn stick_directions_and_scaling() {
        let cfg = MapConfig::default();
        let at = |lx: u16, ly: u16| {
            let x = map_to_xinput(&ProState { lx, ly, ..idle() }, &cfg);
            (x.lx, x.ly)
        };
        assert_eq!(at(2048, 2048), (0, 0));
        assert_eq!(at(2048 + 1100, 2048), (32767, 0)); // right
        assert_eq!(at(2048 - 1100, 2048), (-32767, 0)); // left
        assert_eq!(at(2048, 2048 + 1100), (0, 32767)); // up is positive Y
        assert_eq!(at(2048, 2048 - 1100), (0, -32767)); // down
        assert_eq!(at(4095, 2048), (32767, 0)); // beyond the range clamps
    }

    #[test]
    fn deadzone_swallows_small_motion_and_diagonals_stay_round() {
        let cfg = MapConfig::default();
        let x = map_to_xinput(&ProState { lx: 2048 + 50, ..idle() }, &cfg);
        assert_eq!((x.lx, x.ly), (0, 0));

        let d = map_to_xinput(&ProState { lx: 2048 + 1100, ly: 2048 + 1100, ..idle() }, &cfg);
        assert_eq!(d.lx, d.ly);
        assert!((d.lx as f32).hypot(d.ly as f32) <= 32768.0);
    }

    #[test]
    fn output_reports_have_the_expected_bytes() {
        let mut want = [0u8; OUTPUT_REPORT_LEN];
        want[..7].copy_from_slice(&[0x16, 0x04, 0xA4, 0x00, 0xF0, 0x01, 0x55]);
        assert_eq!(write_register_report(0xA400F0, &[0x55]), want);

        let mut want = [0u8; OUTPUT_REPORT_LEN];
        want[..3].copy_from_slice(&[0x12, 0x04, 0x3D]);
        assert_eq!(report_mode_report(IN_EXT_21), want);

        assert_eq!(led_report(0b0001)[..2], [0x11, 0x10]);
        assert_eq!(status_request_report()[0], 0x15);

        let seq = init_sequence();
        assert_eq!(seq.len(), 4);
        assert_eq!(seq[0][0], 0x16);
        assert_eq!(seq[3][..3], [0x12, 0x04, 0x3D]);
    }

    #[test]
    fn sanitized_config_is_always_usable() {
        let c = MapConfig { nintendo_layout: false, deadzone: f32::NAN, range: -5.0 }.sanitized();
        assert!(c.deadzone.is_finite() && c.range >= 100.0);
        let c = MapConfig { nintendo_layout: false, deadzone: 5.0, range: 1100.0 }.sanitized();
        assert!(c.deadzone <= 0.9);
    }
}
