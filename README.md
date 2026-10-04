# WiiUProTranslator

Use a **Bluetooth Wii U Pro Controller** as a regular **Xbox 360 (XInput) controller** on **Windows 11 ARM64**.

- Written in Rust, builds  for `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc`.
- Reads the controller straight from Windows' own Bluetooth HID stack. No SCP, no DS4Windows, no Wiimote drivers, no x64 emulation.
- Presents a virtual Xbox 360 pad, so every XInput game and launcher sees an ordinary Xbox controller.

## Status

The Wii Remote protocol parsing and the Xbox mapping are unit-tested (`cargo test`). The Windows glue code (Bluetooth HID plus virtual pad) was written against the documented APIs and is built for ARM64 by CI, but it has **not been verified on real hardware yet**. If something misbehaves, please open an issue and include the output of `wiiu-pro-translator --debug` and `wiiu-pro-translator --list`.

## What you need

1. **Windows 11** with a working Bluetooth adapter.
2. **A virtual gamepad driver: [ViGEmBus](https://github.com/nefarius/ViGEmBus/releases)**, version **1.21.442 or newer** (that is the first release with an ARM64 build) which is important if using Windows 11 ARM. Windows has no way to create a virtual XInput controller from a normal program, so some driver is unavoidable; ViGEmBus is the ARM64-capable one, and it is *not* the old SCP driver. Note that ViGEmBus's author archived the project in November 2023, but the installers remain available.
3. This program (download `wiiu-pro-translator-windows-arm64.exe` (or `wiiu-pro-translator-windows-x86_64.exe` for a regular Intel/AMD PC) from the Releases page or the latest Actions run, or build it yourself, see below).

## Pairing the controller

1. Hold the small **SYNC** button on top of the Pro Controller until its LEDs start flashing.
2. Windows **Control Panel → Hardware and Sound → Devices and Printers → Add device**, pick **Nintendo RVL-CNT-01-UC**.
3. If the **Control Panel** redirects you to the **Windows Settings** enter `Control Panel\Hardware and Sound\Devices and Printers` into the URL bar of the Control Panel. 
4. If Windows asks for a code, choose the option to pair just press next **without entering a code**.
5. Once it shows as connected, run the program.

Bluetooth pairing of Nintendo controllers varies between adapters; if it won't stay connected, remove the device in Windows and pair it again.

The program finds the controller by its USB/Bluetooth IDs (vendor `057E`, product `0330`), not by its Windows device name. After pairing, `wiiu-pro-translator.exe --list` should show a line starting with `057e:0330`.

## Usage

```
wiiu-pro-translator.exe [--nintendo-layout] [--deadzone 8] [--range 1100] [--debug] [--list] [--help]
```

Run it from a terminal (PowerShell or Windows Terminal) and leave the window open while you play; closing it unplugs the virtual pad. The program waits for the controller if it isn't connected yet (press a button on the controller to wake it) and reconnects automatically if it drops.

| Option | Meaning |
| --- | --- |
| `--nintendo-layout` | Keep button *labels* (Pro A → Xbox A). Default keeps button *positions* (Pro B, the bottom button → Xbox A), which matches how games expect an Xbox pad to feel. |
| `--deadzone <0-90>` | Radial stick dead zone in percent (default 8). |
| `--range <raw>` | Raw stick travel from centre to edge (default 1100, minimum 100). Raise it if you can't reach full deflection, lower it if the sticks saturate early. |
| `--list` | List every HID device Windows can see, then exit (to check the controller is visible). |
| `--debug` | Print each input change. |
| `-h`, `--help` | Show the built-in help. |

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

With the default layout the face buttons follow their *positions*: Pro B (bottom) → Xbox A, Pro A (right) → Xbox B, Pro Y (left) → Xbox X, Pro X (top) → Xbox Y. With `--nintendo-layout` each button keeps its label instead.

Rumble and battery level are not forwarded yet.

## Troubleshooting

| Message / symptom | What to do |
| --- | --- |
| `Could not open the ViGEmBus driver` | Install [ViGEmBus](https://github.com/nefarius/ViGEmBus/releases) 1.21.442 or newer (the ARM64 build) and run the program again. |
| `Waiting for a paired Wii U Pro Controller...` | The controller isn't connected. Press a button to wake it, or re-pair it (see above). Run `--list` and look for `057e:0330`. |
| `Found Wii U Pro Controller ... but could not open it` | Windows sees the controller but refused access. Make sure no other program is using it, then remove and re-pair the device. |
| `Controller session ended: controller stopped responding` | The Bluetooth link dropped. The program reconnects on its own; if it keeps happening, re-pair the controller. |
| Buttons or sticks behave oddly | Run with `--debug` to see the raw state and the resulting Xbox state, and include that output when opening an issue. |

## Building

Requires the Rust toolchain for `aarch64-pc-windows-msvc` and the Visual Studio Build Tools (with the ARM64 components).

```
cargo build --release
cargo test
```

The binary is `target\release\wiiu-pro-translator.exe`. The protocol and mapping code in `src/protocol.rs` has no Windows dependencies, so `cargo test` also runs on other platforms; the program itself only runs on Windows. The GitHub Actions workflow in `.github/workflows/build.yml` runs whenever you publish a release on GitHub. It runs the tests and the release build for both ARM64 (native `windows-11-arm` runner) and x86_64 (`windows-latest`), checks that each result really is an executable of the right architecture, and then attaches both (`wiiu-pro-translator-windows-arm64.exe` and `wiiu-pro-translator-windows-x86_64.exe`) to that release. It does not run on ordinary pushes or pull requests.

## How it works

The Wii U Pro Controller speaks the Wii Remote HID protocol. The program looks for a HID device with Nintendo's vendor ID (`057E`) and the Pro Controller's product ID (`0330`). On connection it performs the unencrypted extension handshake, sets player LED 1 and switches the controller to report mode `0x3D`, which streams the sticks and buttons about 100 times per second. If the controller sends a status report or goes quiet, the setup is sent again; if it stays silent for about ten seconds the session ends and the program goes back to waiting for it. Each report is decoded (`src/protocol.rs`, plain `std`, fully unit-tested), mapped to an Xbox 360 report with dead-zone handling, and sent to a ViGEmBus virtual pad. Bluetooth access goes through [`hidapi`](https://crates.io/crates/hidapi) with its pure-Rust Windows backend and the pad through [`vigem-client`](https://crates.io/crates/vigem-client), so no C/C++ toolchain is involved.

## License

MIT, see [LICENSE](LICENSE).
