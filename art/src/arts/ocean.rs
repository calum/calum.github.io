//! Where the water is. The ocean layer draws it and the fireballs fall into it,
//! so both ask this one function and cannot disagree.

/// How far down the window the waves sit: 0.0 is the top, 1.0 the bottom.
/// `--wave-clearance` in `index.html` has to leave room above this.
pub const LEVEL: f32 = 0.85;

/// How tall the waves are, in rows stretched to match columns.
const WAVE_HEIGHT: f32 = 6.0;

/// The row of the water's surface above column `x` at time `t` seconds, in the
/// stretched rows a sketch's `cell.y` uses (twice the real row), for a window
/// that is `height` stretched rows tall.
pub fn surface(x: f32, t: f32, height: f32) -> f32 {
    let swell = (x * 0.08 + t).sin() * 0.5 + (x * 0.19 - t * 1.3).sin() * 0.25;
    height * LEVEL + swell * WAVE_HEIGHT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_surface_stays_near_the_level_and_moves_with_time() {
        let height = 100.0;
        let mut seen = std::collections::BTreeSet::new();
        for step in 0..200 {
            let (x, t) = (step as f32 * 0.7, step as f32 * 0.05);
            let y = surface(x, t, height);
            assert!(
                (y - height * LEVEL).abs() <= WAVE_HEIGHT * 0.75 + 1e-3,
                "{y}"
            );
            seen.insert(y.to_bits());
        }
        assert!(seen.len() > 50, "the waves move");
    }
}
