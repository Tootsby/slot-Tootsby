const WARMEST: [f32; 3] = [1.0, 0.82, 0.62];

pub const BLUE_LIGHT_MAX: u8 = 9;
pub const EXTRA_DIM_GAIN: f32 = 0.45;

pub fn blue_light_gain(step: u8) -> [f32; 3] {
    let t = step.min(BLUE_LIGHT_MAX) as f32 / BLUE_LIGHT_MAX as f32;
    let mut gain = [1.0f32; 3];
    for (g, warm) in gain.iter_mut().zip(WARMEST) {
        *g = 1.0 + (warm - 1.0) * t;
    }
    gain
}

pub fn screen_gain(blue_light: u8, extra_dim: bool) -> [f32; 3] {
    let scale = if extra_dim { EXTRA_DIM_GAIN } else { 1.0 };
    blue_light_gain(blue_light).map(|g| g * scale)
}
