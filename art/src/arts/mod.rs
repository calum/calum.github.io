//! Every animation on the site lives here, one file each, and they all run at
//! the same time, drawn on top of each other.
//!
//! # Add an animation
//!
//! 1. Make `src/arts/comet.rs` (copy `sparkle.rs`, or start from the template
//!    below) and give the struct a new name.
//! 2. Add `mod comet;` below.
//! 3. Add one line to [`all`]: `layers.add::<comet::Comet>();`
//!
//! # Or skip the code: add a `World`
//!
//! The library's ready-made worlds (bouncing balls, gravity, trails, palettes)
//! are one call. Paste the `World` from any of its examples:
//!
//! ```ignore
//! layers.world(World::new().balls(14).size(1.6).gravity(-0.004).mouse_pulls().trails().palette(Palette::ocean()));
//! ```
//!
//! Animations are drawn in the order they are listed, so a later one paints
//! over an earlier one wherever they overlap. To switch one off, comment out
//! its line in [`all`]; to remove it, delete its file, `mod` line and `add` line.
//!
//! # The rules of the canvas
//!
//! - It sits over the whole window. Clicks, scrolling and typing still reach the
//!   page underneath, and arrive here too.
//! - `display` is empty at the start of every frame. Whatever you do not draw
//!   shows the page (and its blue background) underneath.
//! - Coordinates are cells, `(0, 0)` at the top left. `frame.cols` and
//!   `frame.rows` are the size of the window in cells, and can change.
//! - Only the character and its foreground colour are drawn. Use `'█'` for a solid block.
//! - Colours are the 16 ANSI ones: `Colour::BrightYellow`, `Colour::Red`, ...
//! - Each animation keeps its own state. They cannot see each other.
//!
//! # A template: a dot drifting across the top row
//!
//! This compiles (it is a test), so it is safe to copy.
//!
//! ```
//! use site_art::{Art, Colour, Display, Frame};
//!
//! pub struct Drifter {
//!     x: f32,
//! }
//!
//! impl Art for Drifter {
//!     fn new(_cols: u32, _rows: u32, _seed: u32) -> Drifter {
//!         Drifter { x: 0.0 }
//!     }
//!
//!     // optional: the window changed size
//!     fn resize(&mut self, _cols: u32, _rows: u32) {}
//!
//!     fn frame(&mut self, display: &mut Display, frame: &Frame) {
//!         self.x = (self.x + 10.0 * frame.dt) % frame.cols as f32;
//!         display.set_pixel(self.x as isize, 0, 'o', Colour::BrightCyan, Colour::Black);
//!     }
//! }
//! ```

use crate::{Layers, Palette, World};

mod sparkle;

/// The animations on the page, first drawn first.
pub fn all(mut layers: Layers) -> Layers {
    layers.add::<sparkle::Sparkle>();

    // Fire balls!
    layers.world(
        World::new()
            .balls(4)
            .size(3.0)
            .gravity(0.0)
            .trails()
            .palette(Palette::fire()),
    );

    // An ocean. The mouse makes a glow, and the waves roll on.
    layers.sketch(Palette::ocean(), |cell| {
        let swell =
            (cell.x * 0.08 + cell.t).sin() * 0.5 + (cell.x * 0.19 - cell.t * 1.3).sin() * 0.25;
        let surface = cell.height * 0.85 + swell * 6.0;
        let glow = (-(cell.dist_to_mouse() * 0.12)).exp();
        let shade = ((surface - cell.y) * 0.25 + glow * 0.5).clamp(-1.0, 1.0);
        // On a terminal the sky is solid white; on the page that would hide
        // everything, so the brightest shades are left see-through.
        if shade > 0.8 {
            -1.0
        } else if shade < 0.0 {
            -0.62 // below the waves a solid deep blue
        } else {
            shade
        }
    });
    layers
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Colour, Machine};
    use std::collections::BTreeSet;

    /// The foreground colours (as palette indexes) of every sparkle glyph on screen.
    fn sparkle_colours(palette: Palette) -> BTreeSet<u32> {
        let mut layers = Layers::new(80, 30, 5);
        layers.add::<sparkle::Sparkle>();
        layers.world(World::new().balls(4).size(3.0).trails().palette(palette));
        let mut machine: Machine<Layers> = Machine::from_art(layers, 80, 30);
        for frame in 0..60 {
            machine.pointer(0, 20.0 + frame as f32, 15.0);
            machine.tick(34.0);
        }
        machine
            .cells
            .chunks(2)
            .filter(|cell| ['*', '+', '.'].contains(&char::from_u32(cell[0]).unwrap()))
            .map(|cell| cell[1] & 255)
            .collect()
    }

    #[test]
    fn a_worlds_palette_does_not_change_the_sparkle_colour() {
        let yellow = Colour::ALL
            .iter()
            .position(|c| *c == Colour::BrightYellow)
            .unwrap() as u32;
        for palette in [
            Palette::mono(),
            Palette::fire(),
            Palette::ocean(),
            Palette::rainbow(),
        ] {
            assert_eq!(
                sparkle_colours(palette.clone()),
                BTreeSet::from([yellow]),
                "{palette:?}"
            );
        }
    }
}
