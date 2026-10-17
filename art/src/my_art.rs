//! YOUR ART GOES HERE.
//!
//! This starter is a small placeholder that shows every input working: sparks
//! follow the pointer, a click throws a burst, the wheel and the page scroll
//! push them around, and any key changes their colour. Delete it all and make
//! something wonderful. You only have to keep the three things the
//! `Art` trait asks for: `new`, `frame`, and (optionally) `resize`.
//!
//! The rules of the canvas:
//!
//! - It sits over the whole window. Clicks, scrolling and typing still reach the
//!   page underneath, and arrive here too.
//! - `display` is empty at the start of every frame. Whatever you do not draw
//!   shows the page (and its blue background) underneath.
//! - Coordinates are cells, `(0, 0)` at the top left. `frame.cols` and
//!   `frame.rows` are the size of the window in cells, and can change.
//! - Only the character and its foreground colour are drawn. Use `'█'` for a solid block.
//! - Colours are the 16 ANSI ones: `Colour::BrightYellow`, `Colour::Red`, ...

use crate::{Art, Colour, Display, Frame, KeyCode, Rng};

/// How many sparks can be alive at once.
const MAX_SPARKS: usize = 400;

/// The colours a key press cycles through.
const COLOURS: [Colour; 5] = [
    Colour::BrightYellow,
    Colour::BrightMagenta,
    Colour::BrightCyan,
    Colour::BrightGreen,
    Colour::White,
];

struct Spark {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
}

pub struct MyArt {
    sparks: Vec<Spark>,
    rng: Rng,
    colour: usize,
}

impl MyArt {
    fn spawn(&mut self, x: f32, y: f32, speed: f32) {
        if self.sparks.len() < MAX_SPARKS {
            self.sparks.push(Spark {
                x,
                y,
                vx: self.rng.range(-speed, speed),
                vy: self.rng.range(-speed, speed) * 0.5,
                life: self.rng.range(0.6, 1.4),
            });
        }
    }
}

impl Art for MyArt {
    fn new(_cols: u32, _rows: u32, seed: u32) -> MyArt {
        MyArt {
            sparks: Vec::new(),
            rng: Rng::new(seed),
            colour: 0,
        }
    }

    fn frame(&mut self, display: &mut Display, frame: &Frame) {
        let pointer = frame.pointer;

        // one spark per frame from the pointer, a burst for every click
        self.spawn(pointer.x, pointer.y, 4.0);
        for &(x, y) in &pointer.clicks {
            for _ in 0..40 {
                self.spawn(x, y, 25.0);
            }
        }

        // any key changes the colour
        if !frame.keys.is_empty() {
            self.colour = (self.colour + 1) % COLOURS.len();
        }
        if frame.keys.iter().any(|k| k.code == KeyCode::Esc) {
            self.sparks.clear();
        }

        // wheel and page scroll push the sparks sideways
        let push = frame.wheel * 2.0;
        let breeze = (frame.scroll - 0.5) * 6.0;

        for spark in &mut self.sparks {
            spark.vy += 8.0 * frame.dt; // gravity
            spark.x += (spark.vx + breeze) * frame.dt + push;
            spark.y += spark.vy * frame.dt;
            spark.life -= frame.dt;
        }
        self.sparks.retain(|s| s.life > 0.0);

        for spark in &self.sparks {
            let glyph = match spark.life {
                l if l > 1.0 => '*',
                l if l > 0.5 => '+',
                _ => '.',
            };
            // set_pixel ignores anything off the screen, so no bounds checks
            display.set_pixel(
                spark.x as isize,
                spark.y as isize,
                glyph,
                COLOURS[self.colour],
                Colour::Black,
            );
        }
    }
}
