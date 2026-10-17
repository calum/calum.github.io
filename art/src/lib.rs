//! The plumbing that runs terminal art over the top of the site.
//!
//! You do not need to read this file to make art: add yours in `arts/` (see
//! `arts/mod.rs`).
//!
//! `overlay.js` (next to `index.html`) loads the compiled WebAssembly, sizes a
//! character grid to the window, and forwards the pointer, wheel, scroll and
//! keyboard. This file holds the grid, calls `MyArt::frame` about 30 times a
//! second, and hands the cells back to be painted on a transparent canvas.
//!
//! Exports (all plain numbers, so no glue code is needed):
//!
//! - `init(cols, rows, seed)`: start up
//! - `resize(cols, rows)`: the window changed size
//! - `tick(ms)`: advance by `ms` milliseconds; 1 if there is a new frame to paint
//! - `pointer(kind, x, y)`: 0 moved, 1 left down, 2 left up, 3 right down,
//!   4 right up, 5 left drag, 6 right drag; `x` and `y` are cells, fractions allowed
//! - `wheel(rows)`, `scroll(fraction)`, `key(code, modifiers)`
//! - `width()`, `height()`, `frame_ptr()`: the cells, two `u32`s each: the
//!   character, then `foreground | background << 8` as palette indexes
//! - `palette(index)`: the `0xRRGGBB` value of a palette colour

mod arts;

pub use terminal_graphics_interactive::{Colour, Display, Key, KeyCode, Modifiers, Pointer};

use std::cell::RefCell;

/// The art runs at about 30 frames a second.
const STEP_MS: f32 = 1000.0 / 30.0;

/// A window bigger than this many cells either way is clamped.
const MAX_CELLS: u32 = 1000;

/// Keys remembered per frame; more than this in 33 ms is not a person typing.
const MAX_KEYS: usize = 32;

/// What the page is doing right now. Handed to [`Art::frame`] every frame.
pub struct Frame<'a> {
    /// Seconds since the page loaded (animation time, not wall-clock).
    pub seconds: f32,
    /// Seconds since the previous frame.
    pub dt: f32,
    /// Grid width in cells (it follows the window).
    pub cols: u32,
    /// Grid height in cells.
    pub rows: u32,
    /// Where the mouse (or finger) is, in cells, and whether buttons are held.
    pub pointer: &'a Pointer,
    /// Keys pressed since the previous frame.
    pub keys: &'a [Key],
    /// Wheel or trackpad scroll since the previous frame, in rows. Down is
    /// positive. Counted even when the page itself cannot scroll.
    pub wheel: f32,
    /// How far down the page is scrolled: 0.0 at the top, 1.0 at the bottom.
    /// Always 0.0 when the page fits the window.
    pub scroll: f32,
}

/// A piece of art. Implement this in `my_art.rs`.
pub trait Art {
    /// Called once. `seed` is different on every page load.
    fn new(cols: u32, rows: u32, seed: u32) -> Self
    where
        Self: Sized;

    /// The window changed size. Keep your state; just fix anything that
    /// depends on the size. Optional.
    fn resize(&mut self, _cols: u32, _rows: u32) {}

    /// Draw one frame. `display` arrives empty: every cell you leave alone
    /// shows the page underneath. Only the character and its colour are used,
    /// the background colour is ignored, so use `'█'` for a solid block.
    fn frame(&mut self, display: &mut Display, frame: &Frame);
}

/// Several arts drawn on top of one another, first to last, so a later one
/// paints over an earlier one. Build it in `arts/mod.rs`.
#[derive(Default)]
pub struct Layers(Vec<Box<dyn Art>>);

impl Layers {
    /// Adds the art `A`. Each layer gets its own seed.
    pub fn add<A: Art + 'static>(&mut self, cols: u32, rows: u32, seed: u32) {
        let seed = seed.wrapping_add(self.0.len() as u32 * 7919);
        self.0.push(Box::new(A::new(cols, rows, seed)));
    }
}

impl Art for Layers {
    fn new(cols: u32, rows: u32, seed: u32) -> Layers {
        arts::all(cols, rows, seed)
    }

    fn resize(&mut self, cols: u32, rows: u32) {
        for layer in &mut self.0 {
            layer.resize(cols, rows);
        }
    }

    fn frame(&mut self, display: &mut Display, frame: &Frame) {
        for layer in &mut self.0 {
            layer.frame(display, frame);
        }
    }
}

/// A tiny random number generator, so the art needs no other crates.
///
/// ```
/// let mut rng = site_art::Rng::new(1);
/// let x = rng.range(10.0, 20.0);
/// assert!((10.0..20.0).contains(&x));
/// ```
#[derive(Clone, Debug)]
pub struct Rng(u32);

impl Rng {
    /// The same seed gives the same numbers.
    pub fn new(seed: u32) -> Rng {
        Rng(seed.max(1))
    }

    /// A number from 0.0 up to (not including) 1.0.
    pub fn next_f32(&mut self) -> f32 {
        // xorshift32
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 >> 8) as f32 / (1u32 << 24) as f32
    }

    /// A number from `low` up to (not including) `high`.
    pub fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.next_f32()
    }
}

/// The art, its grid and the input waiting for the next frame.
pub struct Machine<A: Art> {
    art: A,
    display: Display,
    cells: Vec<u32>,
    pointer: Pointer,
    keys: Vec<Key>,
    wheel: f32,
    scroll: f32,
    seconds: f32,
    waiting_ms: f32,
    cols: u32,
    rows: u32,
}

impl<A: Art> Machine<A> {
    /// A machine with a grid of `cols` by `rows` cells (at least 1 by 1).
    pub fn new(cols: u32, rows: u32, seed: u32) -> Machine<A> {
        let (cols, rows) = (clamp_size(cols), clamp_size(rows));
        Machine::from_art(A::new(cols, rows, seed), cols, rows)
    }

    /// A machine around an art you already made.
    pub fn from_art(art: A, cols: u32, rows: u32) -> Machine<A> {
        let (cols, rows) = (clamp_size(cols), clamp_size(rows));
        Machine {
            art,
            display: Display::new(cols, rows),
            cells: vec![0; (cols * rows * 2) as usize],
            pointer: Pointer::default(),
            keys: Vec::new(),
            wheel: 0.0,
            scroll: 0.0,
            seconds: 0.0,
            waiting_ms: 0.0,
            cols,
            rows,
        }
    }

    /// Follows the window. Does nothing if the size is the same.
    pub fn resize(&mut self, cols: u32, rows: u32) {
        let (cols, rows) = (clamp_size(cols), clamp_size(rows));
        if (cols, rows) == (self.cols, self.rows) {
            return;
        }
        self.cols = cols;
        self.rows = rows;
        self.display = Display::new(cols, rows);
        self.cells = vec![0; (cols * rows * 2) as usize];
        self.art.resize(cols, rows);
        // the old picture is the wrong shape: show a fresh frame straight away
        self.waiting_ms = STEP_MS;
    }

    /// Advances the art by `ms` milliseconds. True if there is a new frame.
    pub fn tick(&mut self, ms: f32) -> bool {
        // NaN and zero do nothing; a long pause (a hidden tab) counts as a short one
        if ms.is_nan() || ms <= 0.0 {
            return false;
        }
        self.waiting_ms += ms.min(250.0);
        if self.waiting_ms < STEP_MS {
            return false;
        }
        let dt = self.waiting_ms / 1000.0;
        self.waiting_ms = 0.0;
        self.seconds += dt;

        self.display.clear();
        self.art.frame(
            &mut self.display,
            &Frame {
                seconds: self.seconds,
                dt,
                cols: self.cols,
                rows: self.rows,
                pointer: &self.pointer,
                keys: &self.keys,
                wheel: self.wheel,
                scroll: self.scroll,
            },
        );
        self.pointer.end_frame();
        self.keys.clear();
        self.wheel = 0.0;
        self.encode();
        true
    }

    /// See the list of `kind`s in the module docs.
    pub fn pointer(&mut self, kind: u32, x: f32, y: f32) {
        if !x.is_finite() || !y.is_finite() {
            return;
        }
        self.pointer.x = x;
        self.pointer.y = y;
        match kind {
            1 => {
                self.pointer.left = true;
                self.pointer.clicks.push((x, y));
            }
            2 => self.pointer.left = false,
            3 | 6 => self.pointer.right = true,
            4 => self.pointer.right = false,
            // a drag proves the button is down even if the press was missed
            5 => self.pointer.left = true,
            _ => {}
        }
    }

    /// Wheel movement in rows; it adds up until the next frame.
    pub fn wheel(&mut self, rows: f32) {
        if rows.is_finite() {
            self.wheel += rows;
        }
    }

    /// Page scroll position, 0.0 to 1.0.
    pub fn scroll(&mut self, fraction: f32) {
        if fraction.is_finite() {
            self.scroll = fraction.clamp(0.0, 1.0);
        }
    }

    /// A key press, as `overlay.js` encodes it (see [`decode_key`]).
    pub fn key(&mut self, code: u32, modifiers: u32) {
        let Some(code) = decode_key(code) else {
            return;
        };
        if self.keys.len() < MAX_KEYS {
            self.keys.push(Key {
                code,
                modifiers: Modifiers {
                    shift: modifiers & 1 != 0,
                    alt: modifiers & 2 != 0,
                    ctrl: modifiers & 4 != 0,
                },
            });
        }
    }

    /// Copies the display into the cells the page reads.
    fn encode(&mut self) {
        for y in 0..self.rows {
            for x in 0..self.cols {
                let index = ((y * self.cols + x) * 2) as usize;
                let (character, colours) = match self.display.pixel(x as isize, y as isize) {
                    Some(pixel) => (
                        pixel.character() as u32,
                        palette_index(pixel.colour()) | palette_index(pixel.background()) << 8,
                    ),
                    None => (' ' as u32, 0),
                };
                self.cells[index] = character;
                self.cells[index + 1] = colours;
            }
        }
    }
}

fn clamp_size(cells: u32) -> u32 {
    cells.clamp(1, MAX_CELLS)
}

fn palette_index(colour: Colour) -> u32 {
    Colour::ALL
        .iter()
        .position(|candidate| *candidate == colour)
        .unwrap_or(0) as u32
}

/// Codes below 0x110000 are the character itself. Above that:
/// 1 Enter, 2 Tab, 3 Backspace, 4 Esc, 5 Up, 6 Down, 7 Left, 8 Right, 9 Home,
/// 10 End, 11 PageUp, 12 PageDown, 13 Insert, 14 Delete, 21 to 32 F1 to F12.
pub fn decode_key(code: u32) -> Option<KeyCode> {
    const NAMED: u32 = 0x11_0000;
    if code < NAMED {
        return char::from_u32(code).map(KeyCode::Char);
    }
    Some(match code - NAMED {
        1 => KeyCode::Enter,
        2 => KeyCode::Tab,
        3 => KeyCode::Backspace,
        4 => KeyCode::Esc,
        5 => KeyCode::Up,
        6 => KeyCode::Down,
        7 => KeyCode::Left,
        8 => KeyCode::Right,
        9 => KeyCode::Home,
        10 => KeyCode::End,
        11 => KeyCode::PageUp,
        12 => KeyCode::PageDown,
        13 => KeyCode::Insert,
        14 => KeyCode::Delete,
        n @ 21..=32 => KeyCode::F((n - 20) as u8),
        _ => return None,
    })
}

// ---------------------------------------------------------------- exports

thread_local! {
    static MACHINE: RefCell<Machine<Layers>> = RefCell::new(Machine::new(80, 24, 1));
}

#[no_mangle]
pub extern "C" fn init(cols: u32, rows: u32, seed: u32) {
    MACHINE.with(|m| *m.borrow_mut() = Machine::new(cols, rows, seed));
}

#[no_mangle]
pub extern "C" fn resize(cols: u32, rows: u32) {
    MACHINE.with(|m| m.borrow_mut().resize(cols, rows));
}

#[no_mangle]
pub extern "C" fn tick(ms: f32) -> u32 {
    MACHINE.with(|m| m.borrow_mut().tick(ms)) as u32
}

#[no_mangle]
pub extern "C" fn pointer(kind: u32, x: f32, y: f32) {
    MACHINE.with(|m| m.borrow_mut().pointer(kind, x, y));
}

#[no_mangle]
pub extern "C" fn wheel(rows: f32) {
    MACHINE.with(|m| m.borrow_mut().wheel(rows));
}

#[no_mangle]
pub extern "C" fn scroll(fraction: f32) {
    MACHINE.with(|m| m.borrow_mut().scroll(fraction));
}

#[no_mangle]
pub extern "C" fn key(code: u32, modifiers: u32) {
    MACHINE.with(|m| m.borrow_mut().key(code, modifiers));
}

#[no_mangle]
pub extern "C" fn width() -> u32 {
    MACHINE.with(|m| m.borrow().cols)
}

#[no_mangle]
pub extern "C" fn height() -> u32 {
    MACHINE.with(|m| m.borrow().rows)
}

#[no_mangle]
pub extern "C" fn frame_ptr() -> usize {
    MACHINE.with(|m| m.borrow().cells.as_ptr() as usize)
}

#[no_mangle]
pub extern "C" fn palette(index: u32) -> u32 {
    let colour = Colour::ALL[index as usize % Colour::ALL.len()];
    let (r, g, b) = colour.rgb();
    (r as u32) << 16 | (g as u32) << 8 | b as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Draws a letter under the pointer and records what the frame saw.
    struct Probe {
        keys: Vec<KeyCode>,
        wheel: f32,
        scroll: f32,
        clicks: usize,
        resized: Option<(u32, u32)>,
    }

    impl Art for Probe {
        fn new(_: u32, _: u32, _: u32) -> Probe {
            Probe {
                keys: Vec::new(),
                wheel: 0.0,
                scroll: 0.0,
                clicks: 0,
                resized: None,
            }
        }
        fn resize(&mut self, cols: u32, rows: u32) {
            self.resized = Some((cols, rows));
        }
        fn frame(&mut self, display: &mut Display, frame: &Frame) {
            self.keys.extend(frame.keys.iter().map(|k| k.code));
            self.wheel += frame.wheel;
            self.scroll = frame.scroll;
            self.clicks += frame.pointer.clicks.len();
            let (x, y) = (frame.pointer.x as isize, frame.pointer.y as isize);
            display.set_pixel(x, y, 'P', Colour::Red, Colour::Blue);
        }
    }

    fn machine() -> Machine<Probe> {
        Machine::new(10, 5, 1)
    }

    fn cell(machine: &Machine<Probe>, x: u32, y: u32) -> (u32, u32) {
        let i = ((y * machine.cols + x) * 2) as usize;
        (machine.cells[i], machine.cells[i + 1])
    }

    #[test]
    fn a_new_machine_is_all_blank_cells_so_the_page_shows_through() {
        let mut m = machine();
        assert!(m.tick(40.0));
        assert!(m.cells.chunks(2).filter(|c| c[0] != ' ' as u32).count() == 1);
    }

    #[test]
    fn colours_are_palette_indexes() {
        let mut m = machine();
        m.pointer(0, 3.4, 2.9);
        m.tick(40.0);
        let (ch, colours) = cell(&m, 3, 2);
        assert_eq!(ch, 'P' as u32);
        assert_eq!(colours & 255, palette_index(Colour::Red));
        assert_eq!(colours >> 8, palette_index(Colour::Blue));
        assert_eq!(
            palette(palette_index(Colour::Red)) >> 16,
            Colour::Red.rgb().0 as u32
        );
    }

    #[test]
    fn frames_come_about_thirty_times_a_second() {
        let mut m = machine();
        assert!(!m.tick(10.0));
        assert!(!m.tick(10.0));
        assert!(m.tick(20.0));
        assert!(!m.tick(0.0));
        assert!(!m.tick(f32::NAN));
        assert!(!m.tick(-5.0));
    }

    #[test]
    fn a_long_pause_counts_as_a_short_one() {
        let mut m = machine();
        m.tick(1_000_000.0);
        assert!(m.seconds <= 0.26, "{}", m.seconds);
    }

    #[test]
    fn a_click_is_seen_by_exactly_one_frame() {
        let mut m = machine();
        m.pointer(1, 4.0, 1.0);
        m.tick(40.0);
        m.tick(40.0);
        assert_eq!(m.art.clicks, 1);
        assert!(m.pointer.left);
        m.pointer(2, 4.0, 1.0);
        assert!(!m.pointer.left);
    }

    #[test]
    fn right_button_and_drags_set_the_buttons() {
        let mut m = machine();
        m.pointer(3, 0.0, 0.0);
        assert!(m.pointer.right);
        m.pointer(4, 0.0, 0.0);
        assert!(!m.pointer.right);
        m.pointer(5, 0.0, 0.0);
        assert!(m.pointer.left);
        m.pointer(6, 0.0, 0.0);
        assert!(m.pointer.right);
    }

    #[test]
    fn nonsense_numbers_are_ignored() {
        let mut m = machine();
        m.pointer(0, 2.0, 2.0);
        m.pointer(0, f32::NAN, 1.0);
        m.pointer(99, f32::INFINITY, 1.0);
        m.wheel(f32::NAN);
        m.scroll(f32::NAN);
        m.key(0xD800, 0); // a surrogate is not a character
        m.key(0x11_0000 + 99, 0);
        assert_eq!((m.pointer.x, m.pointer.y), (2.0, 2.0));
        assert_eq!((m.wheel, m.scroll, m.keys.len()), (0.0, 0.0, 0));
    }

    #[test]
    fn wheel_adds_up_and_scroll_is_a_fraction() {
        let mut m = machine();
        m.wheel(2.0);
        m.wheel(1.5);
        m.scroll(7.0);
        m.tick(40.0);
        assert_eq!((m.art.wheel, m.art.scroll), (3.5, 1.0));
        m.tick(40.0);
        assert_eq!(m.art.wheel, 3.5, "wheel resets after a frame");
    }

    #[test]
    fn keys_are_decoded_and_cleared_each_frame() {
        let mut m = machine();
        m.key('a' as u32, 0);
        m.key(0x11_0000 + 5, 1 | 4);
        m.key(0x11_0000 + 22, 0);
        assert_eq!(
            m.keys[1].modifiers,
            Modifiers {
                shift: true,
                alt: false,
                ctrl: true
            }
        );
        m.tick(40.0);
        assert_eq!(m.art.keys, [KeyCode::Char('a'), KeyCode::Up, KeyCode::F(2)]);
        assert!(m.keys.is_empty());
    }

    #[test]
    fn a_flood_of_keys_is_capped() {
        let mut m = machine();
        for _ in 0..1000 {
            m.key('x' as u32, 0);
        }
        assert_eq!(m.keys.len(), MAX_KEYS);
    }

    #[test]
    fn resize_changes_the_grid_tells_the_art_and_ignores_no_change() {
        let mut m = machine();
        m.resize(10, 5);
        assert_eq!(m.art.resized, None);
        m.resize(20, 8);
        assert_eq!(m.art.resized, Some((20, 8)));
        assert_eq!(m.cells.len(), 20 * 8 * 2);
        assert!(m.tick(1.0), "a resized grid repaints at once");
        assert_eq!((m.display.width(), m.display.height()), (20, 8));
    }

    #[test]
    fn sizes_are_kept_sane() {
        let mut m = machine();
        m.resize(0, 5_000_000);
        assert_eq!((m.cols, m.rows), (1, MAX_CELLS));
    }

    #[test]
    fn drawing_off_the_grid_is_harmless() {
        let mut m = machine();
        m.pointer(0, 5000.0, -40.0);
        assert!(m.tick(40.0));
    }

    #[test]
    fn the_random_numbers_stay_in_range_and_repeat_for_a_seed() {
        let mut a = Rng::new(0);
        let mut b = Rng::new(0);
        for _ in 0..1000 {
            let n = a.next_f32();
            assert!((0.0..1.0).contains(&n));
            assert_eq!(n, b.next_f32());
        }
    }

    /// Fills the top row with its letter.
    struct Row(char);

    impl Art for Row {
        fn new(_: u32, _: u32, _: u32) -> Row {
            Row('?')
        }
        fn frame(&mut self, display: &mut Display, frame: &Frame) {
            for x in 0..frame.cols as isize {
                display.set_pixel(x, 0, self.0, Colour::Red, Colour::Black);
            }
        }
    }

    #[test]
    fn layers_draw_in_order_and_later_ones_paint_over_earlier_ones() {
        let mut layers = Layers::default();
        layers.add::<Row>(10, 5, 1);
        layers.0.push(Box::new(Probe::new(10, 5, 1)));
        let mut m: Machine<Layers> = Machine::from_art(layers, 10, 5);
        m.pointer(0, 2.0, 0.0);
        m.tick(40.0);
        let first = |x| m.cells[(x * 2) as usize];
        assert_eq!(first(1), '?' as u32, "the first layer is still there");
        assert_eq!(first(2), 'P' as u32, "the second paints over it");
    }

    #[test]
    fn every_layer_is_told_about_a_resize() {
        let mut layers = Layers::default();
        layers.0.push(Box::new(Probe::new(10, 5, 1)));
        layers.0.push(Box::new(Probe::new(10, 5, 1)));
        let mut m: Machine<Layers> = Machine::from_art(layers, 10, 5);
        m.resize(30, 9);
        assert!(m.tick(40.0)); // both layers drew on the new grid without a panic
    }

    #[test]
    fn layers_get_different_seeds() {
        struct Seed(u32);
        impl Art for Seed {
            fn new(_: u32, _: u32, seed: u32) -> Seed {
                Seed(seed)
            }
            fn frame(&mut self, display: &mut Display, frame: &Frame) {
                display.set_pixel(
                    self.0 as isize % frame.cols as isize,
                    0,
                    's',
                    Colour::Red,
                    Colour::Black,
                );
            }
        }
        let mut layers = Layers::default();
        layers.add::<Seed>(10_000, 1, 0);
        layers.add::<Seed>(10_000, 1, 0);
        let mut m: Machine<Layers> = Machine::from_art(layers, 1000, 1);
        m.tick(40.0);
        assert_eq!(m.cells.chunks(2).filter(|c| c[0] == 's' as u32).count(), 2);
    }

    #[test]
    fn the_real_art_runs() {
        let mut m: Machine<Layers> = Machine::new(120, 40, 7);
        for _ in 0..100 {
            m.tick(33.4);
        }
        m.pointer(1, 10.0, 10.0);
        m.key('x' as u32, 0);
        m.wheel(3.0);
        assert!(m.tick(40.0));
    }
}
