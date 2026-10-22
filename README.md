# slot (RG SP Custom Edition)

A bespoke, Game Boy-centric frontend for the **Anbernic RG SP** (`rgsp`), forked from [Brandon Kowalski's `slot`](https://github.com/BrandonKowalski/slot) (`v1.8.0`).

Designed specifically around the 720×480 3:2 IPS panel and clamshell form factor of the Anbernic RG SP, with support for **GBA**, **GBC**, and **GB** cartridges.

> [!IMPORTANT]
> ROMs must be unzipped. `slot` does not read `.zip` or `.7z` files.

A full user guide for the base project can be found at [slot-cfw.fyi](https://slot-cfw.fyi), and full release notes are in the [changelog](CHANGELOG.md).

---

# What does it look like?

<p align="center">
<img src=".github/resources/screenshots/carousel.gif" width="48%" alt="Scrolling the cart carousel across the Game Boy Advance, Game Boy and Game Boy Color shelves">
<img src=".github/resources/screenshots/core-picker.gif" width="48%" alt="Flipping a cart to its circuit board to choose mGBA or gpSP">
</p>

<p align="center">
<img src=".github/resources/screenshots/link.gif" width="96.5%" alt="Two SPs racing in Mario Kart over the emulated link">
</p>

---

# Fork Additions & RG SP Enhancements

On top of upstream `slot v1.8.0`, this fork adds hardware-specific display, latency, power, and aesthetic improvements tailored for the **Anbernic RG SP**:

### Display, Shaders & Late-Night Play
- **120 Hz Display Scanout & Native Core Frame Pacing**: Drives the RG SP's panel at `120 Hz` during gameplay and paces the emulation loop at native GBA/GB refresh (`59.7275 Hz`, `16,742,706 ns`) instead of `60.0 Hz`, cutting scanout latency in half (`4.2 ms` avg vs `8.3 ms`) and eliminating periodic audio/video judder. Automatically switches back to `60 Hz` on the cartridge shelf to save power.
- **RG SP Hardware Display Calibration LUT**: Applies a 33×33×33 3D color calibration cube (`rgsp.cube`) folded into a zero-cost `65,536`-entry `RGB565 -> XRGB8888` lookup table at framebuffer present time, correcting the uncalibrated RG SP panel's color cast across both the shelf UI and in-game rendering.
- **Granular 15-Step Brightness (`1/255` Floor) + Extra Dim Filter**: Expanded brightness control from 10 to **15 steps** along a smooth perceptual power curve (`1.9`) starting at hardware minimum `1/255`, paired with a toggleable **Extra Dim** software filter under **Settings → Screen → General** and the in-game **Screen** menu for comfortable late-night play.
- **Sharp-Shimmerless Shader**: Built-in `Sharp-Shimmerless` scaling filter (**Screen → GBA / GB / GBC → Shader**) for crisp, shimmer-free non-integer scaling alongside `LCD3x`, `Grid`, `Dot`, and `Simpletex`.
- **Interframe Blending**: Toggleable per-platform frame blending (**Settings → Gameplay → Interframe Blending**) to smooth out 30 Hz transparency flicker in classic GBA/GB titles without corrupting the paused in-game menu backdrop.

### Battery Life & Performance
- **Software Standby (`10 FPS` Idle Throttling)**: Automatically drops the shelf/menu loop to `10 FPS` after 30 seconds of inactivity and restores full frame rate on the first button press.
- **System-Level Power Savings**: Switches the H700 CPU scaling governor from `performance` to `schedutil`, powers down unused USB host controllers (`ohci1-3`, `ehci1-3`), and unloads the `8821cs` Wi-Fi kernel module after boot when idle—automatically reloading it via `modprobe` whenever a Link session is started.
- **Emulation Loop Micro-Optimizations**: Reuses audio sample buffers across frames (`recycle_audio`) and widens RGB565 pixels two at a time in 32-bit registers before shader passes.

### Controls, Quality-of-Life & Bundled Wallpapers
- **Expanded Controls & Shortcuts**: Supports **Alternate A/B** and **Alternate L/R** mappings, in-game PNG screenshots via `SELECT + X` (saved to `Screenshots/`), and a **Lid Power Off → Off** option so closing the clamshell sleeps indefinitely without forced shutdown.
- **6 Custom Skeuomorphic Wallpapers (`720×480`)**: Ships in `Wallpapers/` with 3 minimalist sticker-free surfaces (`Dark Walnut`, `Charcoal Ebony`, `Matte Slate`) and 3 warm wood shelves with organized retro desktop clutter (`Tinkerer Workbench`, `Collector Desk`, `Late Night Study`).

---

# Supported Devices

While upstream `slot` supports several H700 handhelds, **this fork is specifically tuned and tested for the Anbernic RG SP (`BASEOS_TARGET=rgsp`)**.

| Device             | Target Status in This Fork                   | Resolution |
|--------------------|----------------------------------------------|------------|
| **Anbernic RG SP** | **Primary Target (120 Hz + Display LUT)**    | 720 × 480  |
| Anbernic RG34XX    | Compatible (Upstream Support)                | 720 × 480  |
| Anbernic RG34XXSP  | Compatible (Includes Display LUT)            | 720 × 480  |
| Anbernic RG35XXSP  | Compatible (Upstream Support)                | 640 × 480  |
| Anbernic 35XX Plus | Compatible (Upstream Support)                | 640 × 480  |
| Anbernic RG28XX    | Compatible (Upstream Support)                | 640 × 480  |
| Anbernic RG35XX-H  | Compatible (Upstream Support)                | 640 × 480  |

---

# Credits & Acknowledgements

This fork builds upon the work of several talented developers and artists in the handheld community:

- **[Brandon Kowalski (`@BrandonKowalski`)](https://github.com/BrandonKowalski/slot)** — Creator of [`slot`](https://slot-cfw.fyi), its architecture, cartridge shelf experience, link play integration, and documentation.
- **[Markus (`@MarkusMaribu`)](https://github.com/MarkusMaribu)** — Artist behind `slot`'s skeuomorphic cartridge shelf, slot hardware, button legends, and platform bezels.
- **[Vaibhav (`@pvaibhav`)](https://github.com/pvaibhav/slot)** — Creator of [BaseOS](https://github.com/pvaibhav/baseos) for H700 devices, and author of the RG SP 120 Hz display mode switching, native `59.7275 Hz` frame pacing, 3D LUT hardware display calibration (`rgsp.cube`), `schedutil`/USB/Wi-Fi power management, and emulation loop optimizations adapted into this fork.
- **[`@suprsomething`](https://github.com/suprsomething/slot.advance)** — Author of [`slot.advance`](https://github.com/suprsomething/slot.advance), whose software standby (`10 FPS` idle throttling) and power-saving investigations informed this fork.
- **Emulation & Shaders** — The **mGBA** (endrift / Jeffrey Pfau, MPL-2.0) and **gpSP** (Exophase, davidgfnet & libretro contributors, GPL-2.0) teams, plus the libretro shader authors (`LCD3x`, `Sharp-Shimmerless`, `Simpletex`).

---

# Upstream AI Disclosure & Philosophy

From Brandon Kowalski's original README:

> The Rust frontend, and the SVGs on [slot-cfw.fyi](https://slot-cfw.fyi), were put together by Claude.
> I reviewed everything that was produced. All documentation is 100% free-range, meatbag prose.
>
> My goal with this project was to recreate the feeling of playing my GBA SP in the back seat as a kid,
> but with a few modern creature comforts.
>
> mGBA and gpSP have been around forever and are the true stars of this show. The incredible skeuomorphic
> art is the work of [Markus](https://github.com/MarkusMaribu), an artist who saw the bare-bones version
> and offered to dress it up.
>
> If you enjoy using it and feel inclined to pay it forward, go do something nice for someone! ❤️
