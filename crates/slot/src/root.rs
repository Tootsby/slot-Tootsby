use std::path::{Path, PathBuf};

pub const DIRS: [&str; 28] = [
    "BIOS",
    "Bezels",
    "Bezels/GB",
    "Bezels/GBC",
    "Config",
    "Games",
    "Games/GBA",
    "Games/GB",
    "Games/GBC",
    "Labels",
    "Labels/GBA",
    "Labels/GB",
    "Labels/GBC",
    "Logs",
    "Saves",
    "Saves/GBA",
    "Saves/GB",
    "Saves/GBC",
    "Screenshots",
    "Screenshots/GBA",
    "Screenshots/GB",
    "Screenshots/GBC",
    "States",
    "States/GBA",
    "States/GB",
    "States/GBC",
    "System",
    "Wallpapers",
];

pub const README_FILES: [(&str, &str); 3] = [
    ("Labels/README.txt", LABELS_README),
    ("Wallpapers/README.txt", WALLPAPERS_README),
    ("Bezels/README.txt", BEZELS_README),
];

const LABELS_README: &str = "\
Custom Cartridge Labels & Shells
================================

Label Images:
  Place PNG files (.png) in Labels/GBA/, Labels/GB/, or Labels/GBC/ named
  after your ROM file (for example, Labels/GBA/Metroid Fusion.png matches
  Games/GBA/Metroid Fusion.gba).

  Images are scaled to fill the label well and trimmed evenly from the edges:
  - GBA: 1.94:1 aspect ratio (recommended: 1280x661; internal box: 264x136,
    x 48..312, y 50..186 of 360x212)
  - GB & GBC: 1.13:1 aspect ratio (recommended: 1024x901; internal box: 223x199,
    x 44..266, y 95..293 on notched GB and x 45..267, y 110..308 on rounded GBC
    of 312x337)

Custom Shell Colors (cart_shell.txt):
  Create Labels/cart_shell.txt (or Config/cart_shell.txt) with one line per game:
    <ROM Name> = <outline> <RRGGBB> <finish>
  where:
    <outline> : auto | notched | rounded
    <RRGGBB>  : 6-digit hex color (e.g. 2e8b57)
    <finish>  : solid | clear | glitter
";

const WALLPAPERS_README: &str = "\
Custom Wallpapers
=================

Place PNG files (.png) in this folder and select them under:
  Settings -> General -> Wallpaper

Dimensions & Format:
  - Format: 8-bit RGB, RGBA, or Grayscale PNG (non-indexed)
  - Canvas size: 720x480 (or 640x480 for 4:3 displays like the RG35XXSP)
  - Other sizes are scaled to cover 720x480 and cropped evenly from the center.
  - On 640x480 panels, 40 pixels on the left and right edges are outside the
    frame, so keep important artwork within the center 640x480 area.
";

const BEZELS_README: &str = "\
Custom Game Boy & Game Boy Color Bezels
=======================================

Place PNG files (.png) in Bezels/GB/ or Bezels/GBC/ and select them under:
  Settings -> Screen -> Game Boy (or Game Boy Color) -> Bezel

Dimensions & Format:
  - Format: RGBA PNG with a transparent center window for the game screen
  - Canvas size: 720x480
  - Game screen window: 480x432 starting at x = 120, y = 24
    (120 pixels from the left edge, 24 pixels from the top edge).
  - On 640x480 panels (such as the RG35XXSP), 40 pixels on the left and right
    edges are trimmed, leaving 80-pixel side borders around the 480x432 game.
";

pub fn ensure(root: &Path) {
    for sub in DIRS {
        let _ = std::fs::create_dir_all(root.join(sub));
    }
    for (rel, body) in README_FILES {
        let path = root.join(rel);
        if !path.exists() {
            let _ = std::fs::write(path, body);
        }
    }
    if let Err(e) = slot_store::move_config(root) {
        eprintln!("slot: config: {e}");
    }
}

pub fn bios_dir(root: &Path) -> PathBuf {
    root.join("BIOS")
}

const BIOS_FILE: &str = "gba_bios.bin";

const BIOS_BYTES: u64 = 16 * 1024;
const BIOS_FIRST_BYTE: u8 = 0x18;

pub fn has_real_bios(root: &Path) -> bool {
    let Ok(mut f) = std::fs::File::open(bios_dir(root).join(BIOS_FILE)) else {
        return false;
    };
    if !f.metadata().is_ok_and(|m| m.len() == BIOS_BYTES) {
        return false;
    }
    let mut first = [0u8; 1];
    std::io::Read::read_exact(&mut f, &mut first).is_ok() && first[0] == BIOS_FIRST_BYTE
}

pub fn logs_dir(root: &Path) -> PathBuf {
    root.join("Logs")
}

pub fn saves_dir(root: &Path) -> PathBuf {
    root.join("Saves")
}

pub fn screenshots_dir(root: &Path, platform: slot_store::Platform) -> PathBuf {
    root.join("Screenshots").join(platform.dir_name())
}
