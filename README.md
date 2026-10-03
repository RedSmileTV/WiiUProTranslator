# wiiu-pro-xinput

Use a **Bluetooth Wii U Pro Controller** as a regular **Xbox 360 (XInput) controller** on **Windows 11 ARM64**.

- Written in Rust, builds natively for `aarch64-pc-windows-msvc`.
- Reads the controller straight from Windows' own Bluetooth HID stack. No SCP, no DS4Windows, no Wiimote drivers, no x64 emulation.
- Presents a virtual Xbox 360 pad, so every XInput game and launcher sees an ordinary Xbox controller.

## Status

The Wii Remote protocol parsing and the Xbox mapping are unit-tested (`cargo test`). The Windows glue code (Bluetooth HID plus virtual pad) was written against the documented APIs and is built for ARM64 by CI, but it has **not been verified on real hardware yet**. If something misbehaves, please open an issue and include the output of `wiiu-pro-xinput --debug` and `wiiu-pro-xinput --list`.

## What you need

1. **Windows 11 ARM64** with a working Bluetooth adapter.
2. **A virtual gamepad driver: [ViGEmBus](https://github.com/nefarius/ViGEmBus/releases)**, version **1.21.442 or newer** (that is the first release with an ARM64 build). Windows has no way to create a virtual XInput controller from a normal program, so some driver is unavoidable; ViGEmBus is the ARM64-capable one, and it is *not* the old SCP driver. Note that ViGEmBus's author archived the project in November 2023, but the installers remain available.
3. This program (download `wiiu-pro-xinput.exe` from the Releases page or the latest Actions run, or build it yourself, see below).

## Pairing the controller

1. Hold the small **SYNC** button on top of the Pro Controller until its LEDs start flashing.
2. Windows **Settings → Bluetooth & devices → Add device → Bluetooth**, pick **Nintendo RVL-CNT-01-UC**.
3. If Windows asks for a code, choose the option to pair **without a code**.
4. Once it shows as connected, run the program.

Bluetooth pairing of Nintendo controllers varies between adapters; if it won't stay connected, remove the device in Windows and pair it again.

## Usage

```
wiiu-pro-xinput.exe [--nintendo-layout] [--deadzone 8] [--range 1100] [--debug] [--list]
```

Leave the console window open while you play; closing it unplugs the virtual pad. The program waits for the controller if it isn't connected yet and reconnects automatically if it drops.

| Option | Meaning |
| --- | --- |
| `--nintendo-layout` | Keep button *labels* (Pro A → Xbox A). Default keeps button *positions* (Pro B, the bottom button → Xbox A), which matches how games expect an Xbox pad to feel. |
| `--deadzone <0-90>` | Radial stick dead zone in percent (default 8). |
| `--range <raw>` | Raw stick travel from centre to edge (default 1100). Raise it if you can't reach full deflection, lower it if the sticks saturate early. |
| `--list` | List every HID device Windows can see (to check the controller is visible). |
| `--debug` | Print each input change. |

## Mapping

| Wii U Pro | Xbox 360 |
| --- | --- |
| Left / right stick (and clicks) | Left / right stick (L3 / R3) |
| D-pad | D-pad |
| B / A / Y / X (bottom / right / left / top) | A / B / X / Y |
| L / R | LB / RB |
| ZL / ZR | LT / RT (digital: fully pressed or released) |
| + / − | Start / Back |
| Home | Guide |

Rumble and battery level are not forwarded yet.

## Building

Requires the Rust toolchain for `aarch64-pc-windows-msvc` and the Visual Studio Build Tools (with the ARM64 components).

```
cargo build --release
```

The binary is `target\release\wiiu-pro-xinput.exe`. The GitHub Actions workflow in `.github/workflows/build.yml` does exactly this on a native `windows-11-arm` runner, checks that the result really is an ARM64 executable, and attaches it to releases when you push a `v*` tag.

## How it works

The Wii U Pro Controller speaks the Wii Remote HID protocol. On connection the program performs the unencrypted extension handshake, sets player LED 1 and switches the controller to report mode `0x3D`, which streams the sticks and buttons about 100 times per second. Each report is decoded (`src/protocol.rs`, plain `std`, fully unit-tested), mapped to an Xbox 360 report with dead-zone handling, and sent to a ViGEmBus virtual pad. Bluetooth access goes through [`hidapi`](https://crates.io/crates/hidapi) with its pure-Rust Windows backend and the pad through [`vigem-client`](https://crates.io/crates/vigem-client), so no C/C++ toolchain is involved.

## License

MIT, see [LICENSE](LICENSE).
