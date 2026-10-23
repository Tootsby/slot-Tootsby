// The maths behind the "Light" colour correction: halfway between the raw RGB565 picture and
// mGBA's own colour correction (src/platform/libretro/libretro.c). Shared by build.rs, which
// bakes both tables into the binary so nothing is computed on the device, and by the tests,
// which check the baked tables still match this maths.

pub const GBA_GAMMA_ADJ: f32 = 1.0;
pub const GBC_GAMMA_ADJ: f32 = -0.5;

pub fn light_entry(color: usize, gamma_adj: f32) -> u32 {
    const TARGET_GAMMA: f32 = 2.2;
    const DISPLAY_GAMMA_INV: f32 = 1.0 / TARGET_GAMMA;
    const RGB_MAX: f32 = 31.0;
    const RGB_MAX_INV: f32 = 1.0 / RGB_MAX;
    const LUM: f32 = 0.94;
    const CC_R: f32 = 0.82;
    const CC_G: f32 = 0.665;
    const CC_B: f32 = 0.73;
    const CC_RG: f32 = 0.125;
    const CC_RB: f32 = 0.195;
    const CC_GR: f32 = 0.24;
    const CC_GB: f32 = 0.075;
    const CC_BR: f32 = -0.06;
    const CC_BG: f32 = 0.21;

    let adjusted_gamma = TARGET_GAMMA + gamma_adj;
    let r5 = ((color >> 11) & 0x1f) as u32;
    let g6 = ((color >> 5) & 0x3f) as u32;
    let g5 = ((color >> 6) & 0x1f) as u32;
    let b5 = (color & 0x1f) as u32;

    let r_raw = ((r5 << 3) | (r5 >> 2)) as f32;
    let g_raw = ((g6 << 2) | (g6 >> 4)) as f32;
    let b_raw = ((b5 << 3) | (b5 >> 2)) as f32;

    let r_lin = (r5 as f32 * RGB_MAX_INV).powf(adjusted_gamma);
    let g_lin = (g5 as f32 * RGB_MAX_INV).powf(adjusted_gamma);
    let b_lin = (b5 as f32 * RGB_MAX_INV).powf(adjusted_gamma);

    let r_cc = (LUM * (CC_R * r_lin + CC_GR * g_lin + CC_BR * b_lin))
        .max(0.0)
        .powf(DISPLAY_GAMMA_INV)
        .min(1.0);
    let g_cc = (LUM * (CC_RG * r_lin + CC_G * g_lin + CC_BG * b_lin))
        .max(0.0)
        .powf(DISPLAY_GAMMA_INV)
        .min(1.0);
    let b_cc = (LUM * (CC_RB * r_lin + CC_GB * g_lin + CC_B * b_lin))
        .max(0.0)
        .powf(DISPLAY_GAMMA_INV)
        .min(1.0);

    let r_full = ((r_cc * RGB_MAX + 0.5) as u32).min(31);
    let g_full = ((g_cc * RGB_MAX + 0.5) as u32).min(31);
    let b_full = ((b_cc * RGB_MAX + 0.5) as u32).min(31);

    let r_on = ((r_full << 3) | (r_full >> 2)) as f32;
    let g_on = (((g_full << 1) << 2) | ((g_full << 1) >> 4)) as f32;
    let b_on = ((b_full << 3) | (b_full >> 2)) as f32;

    let r_out = (0.5 * (r_raw + r_on)).round() as u32;
    let g_out = (0.5 * (g_raw + g_on)).round() as u32;
    let b_out = (0.5 * (b_raw + b_on)).round() as u32;

    b_out | (g_out << 8) | (r_out << 16)
}
