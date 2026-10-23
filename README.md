# slot (Tootsby fork)

A bespoke, Game Boy-centric frontend for the **Anbernic RG SP** and other H700 handhelds, forked from [Brandon Kowalski's `slot`](https://github.com/BrandonKowalski/slot) (`v1.8.1`).

Supports **GBA**, **GBC**, and **GB** games.

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

# What this fork adds

This is a personal fork of upstream `slot` (currently synced to **v1.8.1**). I play on an Anbernic RG SP, but the fork deliberately avoids device-specific tweaks so it stays easy to keep in sync with upstream and runs anywhere upstream does.

### Screen
- **Lower brightness for night play**: brightness has 15 steps instead of 9, starting from the panel's dimmest lit level (1/255).
- **Extra Dim**: a toggle under **Settings → Screen → General** (and on the in-game Screen page) that darkens the picture further than the backlight can go.
- **Sharp-Shimmerless shader**: an extra shader option for each platform, for crisp scaling without shimmer at non-integer sizes.
- **Frame Blending**: a per-platform row under **Screen → GBA / GBC / GB** that smooths the 30 Hz flicker some games use for transparency.

### Controls
- **Alternate A/B and L/R** buttons that can be bound alongside the main ones.
- **Screenshots** with `SELECT + X` in game, saved to `Screenshots/`.

### Power
- **Lid Power Off → Off**: closing the lid can sleep indefinitely instead of powering off after a set time.
- **Lighter sleep**: while the lid is closed (or POWER was tapped) slot stops drawing frames to the dark screen and only listens for input.

### Wallpapers
- Six bundled 720×480 wallpapers in `Wallpapers/`: three plain surfaces (Dark Walnut, Charcoal Ebony, Matte Slate) and three wood desks with some clutter on them (Tinkerer Workbench, Collector Desk, Late Night Study).

### Small fixes
- Reuses audio buffers between frames instead of allocating new ones.
- A sound device reopened after silence starts with a buffer of silence (upstream issue #125).

---

# Supported Devices

Same as upstream. I only test on the **Anbernic RG SP**.

| Device             | Supported | Since Version |
|--------------------|-----------|---------------|
| Anbernic RG SP     | Yes       | Always        |
| Anbernic RG34XX    | Yes       | 1.4.0         |
| Anbernic RG34XXSP  | Yes       | 1.4.0         |
| Anbernic RG35XXSP  | Yes       | 1.5.0         |
| Anbernic 35XX Plus | Yes       | 1.5.0         |
| Anbernic RG28XX    | Yes       | 1.6.0         |
| Anbernic RG35XX-H  | Yes       | 1.7.0         |

---

# Credits

- **[Brandon Kowalski](https://github.com/BrandonKowalski/slot)**: creator of `slot`. Almost everything here is his work; see [slot-cfw.fyi](https://slot-cfw.fyi).
- **[Markus (@MarkusMaribu)](https://github.com/MarkusMaribu)**: the skeuomorphic shelf, cart and bezel art.
- **[Vaibhav (@pvaibhav)](https://github.com/pvaibhav/slot)**: BaseOS, and the audio buffer reuse adapted from his fork.
- **[@suprsomething](https://github.com/suprsomething/slot.advance)**: the idea of not drawing while dozing, from `slot.advance`.
- **mGBA** (MPL-2.0) and **gpSP** (GPL-2.0), the emulators doing the real work.

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
