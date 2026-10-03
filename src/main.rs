//! wiiu-pro-xinput: Bluetooth Wii U Pro Controller -> virtual Xbox 360 (XInput) pad.

#[cfg(windows)]
mod app;
#[cfg_attr(not(windows), allow(dead_code))]
mod protocol;

#[cfg(windows)]
fn main() {
    app::main();
}

#[cfg(not(windows))]
fn main() {
    eprintln!("wiiu-pro-xinput only runs on Windows (it needs the ViGEmBus virtual gamepad driver).");
    std::process::exit(1);
}
