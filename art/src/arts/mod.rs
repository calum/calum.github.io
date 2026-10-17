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

    // Bubbles float up (gravity is negative). Clicks go to the page, not to the bubbles.
    layers.world(
        World::new()
            .balls(4)
            .size(3.0)
            .gravity(-0.004)
            .trails()
            .palette(Palette::fire()),
    );
    layers
}
