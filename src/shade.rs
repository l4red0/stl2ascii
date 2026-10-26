use crate::cli::ShadeMode;

const DENSITY: &[u8] = b" .:-=+*#%@";
const LINES: &[u8] = b" .'`-_/|\\";

pub fn shade_char(mode: ShadeMode, intensity: f32) -> char {
    let ramp = match mode {
        ShadeMode::Density => DENSITY,
        ShadeMode::Lines => LINES,
    };
    let t = intensity.clamp(0.0, 1.0);
    let idx = ((t * (ramp.len() - 1) as f32) + 0.5) as usize;
    ramp[idx.min(ramp.len() - 1)] as char
}
