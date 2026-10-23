use std::fmt::Write as _;
use std::path::PathBuf;

#[path = "src/light_lut.rs"]
mod light_lut;

// Bake the "Light" colour correction tables into the binary, so switching to Light mid-game
// costs nothing on the device instead of a 65536-entry build on the emulator thread.
fn main() {
    println!("cargo:rerun-if-changed=src/light_lut.rs");
    println!("cargo:rerun-if-changed=build.rs");

    let mut src = String::new();
    for (name, adj) in [
        ("GBA_LIGHT_LUT", light_lut::GBA_GAMMA_ADJ),
        ("GBC_LIGHT_LUT", light_lut::GBC_GAMMA_ADJ),
    ] {
        writeln!(src, "static {name}: [u32; 65536] = [").unwrap();
        for color in 0..65536usize {
            write!(src, "{},", light_lut::light_entry(color, adj)).unwrap();
            if color % 16 == 15 {
                src.push('\n');
            }
        }
        src.push_str("];\n");
    }

    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("light_lut.rs");
    std::fs::write(out, src).unwrap();
}
