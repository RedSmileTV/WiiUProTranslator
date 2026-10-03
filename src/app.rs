//! Windows-only glue: finds the controller over Bluetooth HID (hidapi, pure-Rust
//! backend) and feeds a virtual Xbox 360 pad (ViGEmBus via vigem-client).

use std::process::exit;
use std::thread::sleep;
use std::time::Duration;

use hidapi::{DeviceInfo, HidApi, HidDevice, HidError};
use vigem_client::{Client, TargetId, XButtons, XGamepad, Xbox360Wired};

use crate::protocol::{self, MapConfig, XState};

const HELP: &str = "\
wiiu-pro-xinput - use a Bluetooth Wii U Pro Controller as an Xbox 360 (XInput) pad

USAGE:
    wiiu-pro-xinput [OPTIONS]

OPTIONS:
    --nintendo-layout     Keep face-button labels (Pro A -> Xbox A) instead of
                          positions (Pro B, the bottom button -> Xbox A)
    --deadzone <PERCENT>  Radial stick dead zone, 0-90 (default 8)
    --range <RAW>         Raw stick travel from centre to edge (default 1100);
                          raise it if full deflection is never reached
    --list                List all HID devices Windows can see, then exit
    --debug               Print every input change
    -h, --help            Show this help

Pair the controller in Windows Bluetooth settings first (hold its SYNC button),
and install ViGEmBus (ARM64 build is included in 1.21.442 and later).";

struct Options {
    map: MapConfig,
    debug: bool,
    list: bool,
}

fn next_number(args: &mut impl Iterator<Item = String>, name: &str) -> f32 {
    match args.next().and_then(|v| v.parse::<f32>().ok()) {
        Some(v) => v,
        None => {
            eprintln!("{name} needs a number\n\n{HELP}");
            exit(2)
        }
    }
}

fn parse_args() -> Options {
    let mut opts = Options {
        map: MapConfig::default(),
        debug: false,
        list: false,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("{HELP}");
                exit(0);
            }
            "--nintendo-layout" => opts.map.nintendo_layout = true,
            "--debug" => opts.debug = true,
            "--list" => opts.list = true,
            "--deadzone" => opts.map.deadzone = next_number(&mut args, "--deadzone") / 100.0,
            "--range" => opts.map.range = next_number(&mut args, "--range"),
            other => {
                eprintln!("unknown option: {other}\n\n{HELP}");
                exit(2);
            }
        }
    }
    opts.map = opts.map.sanitized();
    opts
}

fn list_devices(api: &HidApi) {
    for d in api.device_list() {
        println!(
            "{:04x}:{:04x}  {}  {}",
            d.vendor_id(),
            d.product_id(),
            d.product_string().unwrap_or("?"),
            d.path().to_string_lossy()
        );
    }
}

/// The Wii Remote Plus shares the Pro Controller's product id; its product
/// string ends in "-TR" while the Pro Controller's ends in "-UC".
fn is_wiiu_pro(d: &DeviceInfo) -> bool {
    d.vendor_id() == protocol::NINTENDO_VID
        && d.product_id() == protocol::WIIU_PRO_PID
        && d.product_string().map_or(true, |s| s.is_empty() || s.contains("UC"))
}

fn open_controller(api: &mut HidApi) -> Option<HidDevice> {
    if api.refresh_devices().is_err() {
        return None;
    }
    let info = api.device_list().find(|d| is_wiiu_pro(d))?;
    match info.open_device(api) {
        Ok(dev) => Some(dev),
        Err(e) => {
            eprintln!("Found the controller but could not open it: {e}");
            None
        }
    }
}

fn init_controller(dev: &HidDevice) -> Result<(), HidError> {
    for report in protocol::init_sequence() {
        dev.write(&report)?;
        sleep(Duration::from_millis(30));
    }
    Ok(())
}

fn to_gamepad(x: &XState) -> XGamepad {
    XGamepad {
        buttons: XButtons { raw: x.buttons },
        left_trigger: x.left_trigger,
        right_trigger: x.right_trigger,
        thumb_lx: x.lx,
        thumb_ly: x.ly,
        thumb_rx: x.rx,
        thumb_ry: x.ry,
    }
}

/// Stream input from one connected controller until it goes away.
fn run_session(
    dev: &HidDevice,
    opts: &Options,
    push: &mut impl FnMut(&XGamepad) -> Result<(), String>,
) -> Result<(), String> {
    let hid_err = |e: HidError| e.to_string();

    dev.write(&protocol::status_request_report()).map_err(hid_err)?;
    init_controller(dev).map_err(hid_err)?;

    let mut buf = [0u8; 64];
    let mut last = XGamepad::default();
    let mut silent_polls = 0u32;

    loop {
        let n = dev.read_timeout(&mut buf, 500).map_err(hid_err)?;
        if n == 0 {
            // The controller streams ~100 reports/s once initialised, so a
            // quiet link means it dropped back to its default mode or left.
            silent_polls += 1;
            if silent_polls >= 20 {
                return Err("controller stopped responding".into());
            }
            if silent_polls % 2 == 0 {
                init_controller(dev).map_err(hid_err)?;
            }
            continue;
        }
        silent_polls = 0;

        // Account for Windows hidapi prepending a report ID prefix byte (0x00)
        let report_id = if buf[0] != 0 { buf[0] } else { buf[1] };

        match report_id {
            // A status report resets the data reporting mode, so set it up again.
            protocol::IN_STATUS => init_controller(dev).map_err(hid_err)?,
            protocol::IN_EXT_21 => {
                if let Some(state) = protocol::parse_input_report(&buf[..n]) {
                    let x = protocol::map_to_xinput(&state, &opts.map);
                    let pad = to_gamepad(&x);
                    if pad != last {
                        if opts.debug {
                            println!("{state:?} -> {x:?}");
                        }
                        push(&pad)?;
                        last = pad;
                    }
                }
            }
            _ => {}
        }
    }
}

pub fn main() {
    let opts = parse_args();

    let mut api = match HidApi::new() {
        Ok(api) => api,
        Err(e) => {
            eprintln!("Could not initialise Windows HID access: {e}");
            exit(1);
        }
    };
    if opts.list {
        list_devices(&api);
        return;
    }

    let client = match Client::connect() {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "Could not open the ViGEmBus driver ({e:?}).\n\
                 Install ViGEmBus first (the ARM64 build is in the 1.21.442+ installer):\n\
                 https://github.com/nefarius/ViGEmBus/releases"
            );
            exit(1);
        }
    };
    let mut pad = Xbox360Wired::new(client, TargetId::XBOX360_WIRED);
    if let Err(e) = pad.plugin() {
        eprintln!("Could not create the virtual Xbox 360 controller: {e:?}");
        exit(1);
    }
    if let Err(e) = pad.wait_ready() {
        eprintln!("The virtual Xbox 360 controller never became ready: {e:?}");
        exit(1);
    }
    println!("Virtual Xbox 360 controller is plugged in. Leave this window open.");

    let mut push = |g: &XGamepad| pad.update(g).map_err(|e| format!("virtual pad update failed: {e:?}"));

    let mut announced_waiting = false;
    loop {
        match open_controller(&mut api) {
            Some(dev) => {
                announced_waiting = false;
                println!("Wii U Pro Controller connected.");
                if let Err(e) = run_session(&dev, &opts, &mut push) {
                    eprintln!("Controller session ended: {e}");
                }
                // Never leave a button stuck down in the game.
                let _ = push(&XGamepad::default());
                println!("Controller disconnected. Waiting for it to come back...");
            }
            None => {
                if !announced_waiting {
                    println!("Waiting for a paired Wii U Pro Controller (press a button to wake it)...");
                    announced_waiting = true;
                }
                sleep(Duration::from_secs(1));
            }
        }
    }
}