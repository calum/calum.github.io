//! Fireballs: balls that bounce off the walls and each other, leave a fiery
//! trail, and are pulled very gently downwards. A ball that touches the ocean
//! turns grey, can no longer bounce off anything, shrinks away and is replaced
//! by a new one somewhere above the water, going in a random direction.
//!
//! This is the library's `World` physics with the water added. A `World` keeps
//! its balls private, so it cannot recolour, remove or respawn one.
//! Change the numbers just below to change how they behave.

use super::ocean;
use crate::{Art, Colour, Display, Frame, Palette, Rng};

/// How many balls there are, always.
const BALLS: usize = 4;
/// How big they are, in columns.
const RADIUS: f32 = 3.0;
/// How hard they are pulled down, per frame. Very small: they should mostly
/// bounce around. The old bubbles used -0.004 to float up; 0 is none at all.
const GRAVITY: f32 = 0.001;
/// How many frames a trail lasts.
const TRAIL: u8 = 14;
/// How many frames a ball takes to fade away in the water (about 1.5 seconds).
const FADE_FRAMES: u8 = 45;
/// How fast the water slows a ball, and how fast it sinks, per frame.
const WATER_DRAG: f32 = 0.9;
const SINK: f32 = 0.01;

struct Ball {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    /// Frames of life left once it is in the water; `None` while it is flying.
    fading: Option<u8>,
}

pub struct Fireballs {
    balls: Vec<Ball>,
    /// For every cell: frames of trail left, and the speed of the ball that made it.
    tails: Vec<(u8, f32)>,
    palette: Palette,
    rng: Rng,
    cols: u32,
    rows: u32,
    started: bool,
    /// How many balls have been replaced so far.
    respawns: u32,
}

impl Fireballs {
    /// The radius used: the one asked for, shrunk if the window is too small.
    fn radius(&self) -> f32 {
        let (width, height) = (self.cols as f32, self.rows as f32);
        RADIUS.min((width - 1.0) / 2.0).min(height - 1.0).max(0.5)
    }

    /// The range a ball's centre can be in: `(left, right, top, bottom)`.
    fn bounds(&self) -> (f32, f32, f32, f32) {
        let radius = self.radius();
        let (width, height) = (self.cols as f32, self.rows as f32);
        (
            radius,
            (width - 1.0 - radius).max(radius),
            radius / 2.0,
            (height - 1.0 - radius / 2.0).max(radius / 2.0),
        )
    }

    /// A new ball somewhere above the water, going in a random direction.
    fn spawn(&mut self, t: f32) -> Ball {
        let (left, right, top, bottom) = self.bounds();
        let x = self.rng.range(left, right);
        let surface = ocean::surface(x, t, self.rows as f32 * 2.0) / 2.0;
        // keep it clear of the water so it does not drown the moment it appears
        let lowest = (surface - self.radius()).min(bottom).max(top);
        Ball {
            x,
            y: self.rng.range(top, lowest),
            vx: self.rng.range(-0.6, 0.6),
            vy: self.rng.range(-0.3, 0.3),
            fading: None,
        }
    }

    /// One frame of physics, at animation time `t`.
    fn step(&mut self, t: f32) {
        let (left, right, top, bottom) = self.bounds();

        if !self.started {
            self.started = true;
            self.balls = (0..BALLS).map(|_| self.spawn(t)).collect();
        }

        for frames in self.tails.iter_mut().map(|tail| &mut tail.0) {
            *frames = frames.saturating_sub(1);
        }

        let (radius, rows) = (self.radius(), self.rows);
        for ball in &mut self.balls {
            match &mut ball.fading {
                None => {
                    ball.vy += GRAVITY;
                    ball.x += ball.vx;
                    ball.y += ball.vy;
                    if touches_water(ball.x, ball.y, radius, t, rows) {
                        // in the water: from here on it never bounces
                        ball.fading = Some(FADE_FRAMES);
                    } else {
                        // bounce off the walls, losing a little energy
                        if ball.x < left || ball.x > right {
                            ball.vx = -ball.vx * 0.95;
                            ball.x = ball.x.clamp(left, right);
                        }
                        if ball.y < top || ball.y > bottom {
                            ball.vy = -ball.vy * 0.92;
                            ball.y = ball.y.clamp(top, bottom);
                        }
                    }
                }
                Some(frames) => {
                    ball.vx *= WATER_DRAG;
                    ball.vy = ball.vy * WATER_DRAG + SINK;
                    ball.x = (ball.x + ball.vx).clamp(left, right);
                    ball.y = (ball.y + ball.vy).clamp(top, bottom);
                    *frames = frames.saturating_sub(1);
                }
            }
        }

        self.collide();

        for ball in &mut self.balls {
            ball.x = ball.x.clamp(left, right);
            ball.y = ball.y.clamp(top, bottom);
        }

        // out of the water again as a new ball
        for index in 0..self.balls.len() {
            let ball = &self.balls[index];
            let sane = [ball.x, ball.y, ball.vx, ball.vy]
                .iter()
                .all(|n| n.is_finite());
            if !sane || ball.fading == Some(0) {
                self.balls[index] = self.spawn(t);
                self.respawns += 1;
            }
        }

        self.mark_tails();
    }

    /// Flying balls bounce off each other as equal weights. Rows are doubled
    /// so a ball is round. Balls fading in the water pass through.
    fn collide(&mut self) {
        let radius = self.radius();
        for i in 0..self.balls.len() {
            for j in (i + 1)..self.balls.len() {
                let (a, b) = (&self.balls[i], &self.balls[j]);
                if a.fading.is_some() || b.fading.is_some() {
                    continue;
                }
                let (dx, dy) = (b.x - a.x, (b.y - a.y) * 2.0);
                let distance = (dx * dx + dy * dy).sqrt();
                if distance >= 2.0 * radius {
                    continue;
                }
                let (nx, ny) = if distance > 0.0 {
                    (dx / distance, dy / distance)
                } else {
                    (1.0, 0.0)
                };
                let approach = (b.vx - a.vx) * nx + (b.vy - a.vy) * 2.0 * ny;

                // push them apart so they do not stick together
                let overlap = (2.0 * radius - distance) / 2.0;
                self.balls[i].x -= nx * overlap;
                self.balls[i].y -= ny * overlap / 2.0;
                self.balls[j].x += nx * overlap;
                self.balls[j].y += ny * overlap / 2.0;

                if approach < 0.0 {
                    self.balls[i].vx += approach * nx;
                    self.balls[i].vy += approach * ny / 2.0;
                    self.balls[j].vx -= approach * nx;
                    self.balls[j].vy -= approach * ny / 2.0;
                }
            }
        }
    }

    /// Paints each flying ball as a disc of cells into the trails.
    fn mark_tails(&mut self) {
        let radius = self.radius();
        let (width, height) = (self.cols as isize, self.rows as isize);
        for ball in self.balls.iter().filter(|ball| ball.fading.is_none()) {
            let speed = (ball.vx * ball.vx + (ball.vy * 2.0).powi(2)).sqrt();
            for (column, row) in disc(ball.x, ball.y, radius) {
                if (0..width).contains(&column) && (0..height).contains(&row) {
                    self.tails[(row * width + column) as usize] = (TRAIL, speed);
                }
            }
        }
    }

    fn draw(&self, display: &mut Display) {
        // the fiery trails and flying balls
        for row in 0..self.rows {
            for column in 0..self.cols {
                let (frames_left, speed) = self.tails[(row * self.cols + column) as usize];
                if frames_left == 0 {
                    continue;
                }
                let fresh = frames_left as f32 / TRAIL as f32;
                let brightness = (speed / 1.5).clamp(0.15, 1.0) * (0.4 + 0.6 * fresh);
                let (block, foreground, background) = self.palette.shade(brightness);
                // the newest cell of a ball is solid
                let block = if frames_left == TRAIL { '█' } else { block };
                display.set_pixel(column as isize, row as isize, block, foreground, background);
            }
        }
        // balls in the water: grey, solid, shrinking
        for ball in &self.balls {
            let Some(frames) = ball.fading else { continue };
            let left = frames as f32 / FADE_FRAMES as f32;
            let colour = match left {
                l if l > 2.0 / 3.0 => Colour::BrightWhite,
                l if l > 1.0 / 3.0 => Colour::White,
                _ => Colour::BrightBlack,
            };
            // a disc smaller than a cell can fall between the cell centres, so
            // keep the middle cell until the ball is gone
            let mut cells = disc(ball.x, ball.y, self.radius() * left);
            cells.push((ball.x.round() as isize, ball.y.round() as isize));
            for (column, row) in cells {
                display.set_pixel(column, row, '█', colour, Colour::Black);
            }
        }
    }
}

/// Has a ball at `(x, y)` touched the water at time `t`? That is when the
/// bottom edge of the ball reaches the surface, not when its centre does, so a
/// ball never visibly sits in the water while still flying.
fn touches_water(x: f32, y: f32, radius: f32, t: f32, rows: u32) -> bool {
    y * 2.0 + radius > ocean::surface(x, t, rows as f32 * 2.0)
}

/// The cells (column, row) whose centres are inside a ball of `radius` at `(x, y)`.
fn disc(x: f32, y: f32, radius: f32) -> Vec<(isize, isize)> {
    let reach_x = radius.ceil() as isize + 1;
    let reach_y = (radius / 2.0).ceil() as isize + 1;
    let (centre_x, centre_y) = (x.round() as isize, y.round() as isize);
    let mut cells = Vec::new();
    for row in (centre_y - reach_y)..=(centre_y + reach_y) {
        for column in (centre_x - reach_x)..=(centre_x + reach_x) {
            let dx = column as f32 - x;
            let dy = (row as f32 - y) * 2.0;
            if dx * dx + dy * dy <= radius * radius {
                cells.push((column, row));
            }
        }
    }
    cells
}

impl Art for Fireballs {
    fn new(cols: u32, rows: u32, seed: u32) -> Fireballs {
        Fireballs {
            balls: Vec::new(),
            tails: vec![(0, 0.0); (cols * rows) as usize],
            palette: Palette::fire(),
            rng: Rng::new(seed),
            cols,
            rows,
            started: false,
            respawns: 0,
        }
    }

    fn resize(&mut self, cols: u32, rows: u32) {
        self.cols = cols;
        self.rows = rows;
        self.tails = vec![(0, 0.0); (cols * rows) as usize];
    }

    fn frame(&mut self, display: &mut Display, frame: &Frame) {
        // one step per frame, which is how the speeds are tuned
        self.step(frame.seconds);
        self.draw(display);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Layers, Machine};

    const COLS: u32 = 120;
    const ROWS: u32 = 40;

    fn fireballs(seed: u32) -> Fireballs {
        Fireballs::new(COLS, ROWS, seed)
    }

    fn ball(x: f32, y: f32, vx: f32, vy: f32) -> Ball {
        Ball {
            x,
            y,
            vx,
            vy,
            fading: None,
        }
    }

    fn in_water(f: &Fireballs, ball: &Ball, t: f32) -> bool {
        touches_water(ball.x, ball.y, f.radius(), t, f.rows)
    }

    /// A world with just these balls, as if it had already started.
    fn with(balls: Vec<Ball>) -> Fireballs {
        let mut f = fireballs(1);
        f.balls = balls;
        f.started = true;
        f
    }

    /// A row that is under the water at every column and every moment.
    const DEEP: f32 = ROWS as f32 - 3.0;

    // -------------------------------------------------------------- starting

    #[test]
    fn it_starts_with_four_balls_above_the_water() {
        let mut f = fireballs(3);
        f.step(0.0);
        assert_eq!(f.balls.len(), BALLS);
        for ball in &f.balls {
            assert!(ball.fading.is_none());
            assert!(!in_water(&f, ball, 0.0), "{} {}", ball.x, ball.y);
        }
    }

    #[test]
    fn balls_start_where_the_window_really_is_not_in_the_one_cell_window_it_is_made_in() {
        let mut f = Fireballs::new(1, 1, 3);
        f.resize(COLS, ROWS);
        f.step(0.0);
        assert!(f.balls.iter().any(|ball| ball.x > 10.0));
    }

    // --------------------------------------------------------------- gravity

    #[test]
    fn gravity_pulls_down_but_only_a_little() {
        let mut f = with(vec![ball(60.0, 5.0, 0.0, 0.0)]);
        f.step(0.0);
        let first = f.balls[0].vy;
        assert!(first > 0.0, "downwards is positive");
        assert!(first < 0.005, "and it is gentle: {first}");
    }

    #[test]
    fn a_ball_bounces_off_the_ceiling_and_walls() {
        let (_, right, top, _) = fireballs(1).bounds();
        let mut f = with(vec![ball(right, top, 0.5, -0.5)]);
        f.step(0.0);
        assert!(f.balls[0].vx < 0.0 && f.balls[0].vy > 0.0);
    }

    #[test]
    fn balls_that_meet_bounce_off_each_other() {
        let mut f = with(vec![ball(50.0, 8.0, 0.4, 0.0), ball(55.0, 8.0, -0.4, 0.0)]);
        for _ in 0..10 {
            f.step(0.0);
        }
        assert!(
            f.balls[0].vx < 0.0 && f.balls[1].vx > 0.0,
            "they turned round"
        );
        let distance = (f.balls[1].x - f.balls[0].x).abs();
        assert!(distance >= 2.0 * RADIUS - 0.5, "{distance}");
    }

    // ----------------------------------------------------------------- water

    #[test]
    fn a_ball_above_the_water_keeps_flying() {
        let mut f = with(vec![ball(60.0, 3.0, 0.0, 0.0)]);
        f.step(0.0);
        assert_eq!(f.balls[0].fading, None);
    }

    #[test]
    fn a_ball_that_enters_the_water_starts_to_fade() {
        let mut f = with(vec![ball(60.0, DEEP, 0.0, 0.0)]);
        f.step(0.0);
        assert_eq!(f.balls[0].fading, Some(FADE_FRAMES));
        assert_eq!(f.respawns, 0, "it is still there, fading");
    }

    #[test]
    fn a_ball_whose_edge_touches_the_water_has_hit_it_even_if_its_centre_is_still_above() {
        let surface = ocean::surface(60.0, 0.0, ROWS as f32 * 2.0);
        // the centre is 0.5 stretched rows above the surface minus the radius:
        // the bottom edge dips just into the water
        let y = (surface + 0.5 - RADIUS) / 2.0;
        let mut f = with(vec![ball(60.0, y, 0.0, 0.0)]);
        assert!(
            f.balls[0].y * 2.0 < surface,
            "the centre is above the water"
        );
        f.step(0.0);
        assert!(f.balls[0].fading.is_some());
    }

    #[test]
    fn a_ball_that_hits_the_water_at_a_wall_does_not_bounce_off_the_wall() {
        let (_, right, _, _) = fireballs(1).bounds();
        let mut f = with(vec![ball(right - 0.1, DEEP, 0.5, 0.0)]);
        f.step(0.0);
        assert!(f.balls[0].fading.is_some());
        assert!(
            f.balls[0].vx > 0.0,
            "it was not turned back: {}",
            f.balls[0].vx
        );
    }

    #[test]
    fn a_ball_that_hits_the_water_does_not_bounce_off_another_ball() {
        let mut f = with(vec![
            ball(60.0, DEEP, 0.4, 0.0),
            ball(63.0, DEEP, -0.4, 0.0),
        ]);
        f.step(0.0);
        assert!(f.balls.iter().all(|b| b.fading.is_some()));
        assert!(
            f.balls[0].vx > 0.0 && f.balls[1].vx < 0.0,
            "they passed through"
        );
    }

    #[test]
    fn a_flying_ball_passes_through_one_that_is_fading() {
        let mut fading = ball(60.0, DEEP, 0.0, 0.0);
        fading.fading = Some(FADE_FRAMES);
        let mut f = with(vec![fading, ball(62.0, DEEP - 6.0, 0.0, 0.3)]);
        let before = f.balls[1].vy;
        f.step(0.0);
        assert!(
            (f.balls[1].vy - (before + GRAVITY)).abs() < 1e-6,
            "{}",
            f.balls[1].vy
        );
    }

    #[test]
    fn a_fading_ball_never_bounces_off_the_walls() {
        let (left, _, _, bottom) = fireballs(1).bounds();
        let mut at_left = ball(left + 0.2, DEEP, -0.5, 0.0);
        at_left.fading = Some(FADE_FRAMES);
        let mut at_bottom = ball(60.0, bottom - 0.1, 0.0, 0.5);
        at_bottom.fading = Some(FADE_FRAMES);
        let mut f = with(vec![at_left, at_bottom]);
        for _ in 0..FADE_FRAMES - 1 {
            f.step(0.0);
            assert!(f.balls[0].vx <= 0.0, "turned back at the left wall");
            assert!(f.balls[1].vy >= 0.0, "turned back at the bottom");
        }
    }

    #[test]
    fn a_fading_ball_is_grey_solid_and_shrinks() {
        let mut f = with(vec![ball(60.0, DEEP, 0.0, 0.0)]);
        let mut sizes = Vec::new();
        for _ in 0..FADE_FRAMES - 1 {
            f.step(0.0);
            let mut display = Display::new(COLS, ROWS);
            f.draw(&mut display);
            let mut size = 0;
            for y in 0..ROWS as isize {
                for x in 0..COLS as isize {
                    let pixel = display.pixel(x, y).unwrap();
                    if pixel.character() != ' ' {
                        size += 1;
                        assert_eq!(pixel.character(), '█', "solid, so no page shows through");
                        assert!(
                            [Colour::BrightWhite, Colour::White, Colour::BrightBlack]
                                .contains(&pixel.colour()),
                            "{:?}",
                            pixel.colour()
                        );
                    }
                }
            }
            sizes.push(size);
        }
        assert!(sizes[0] > sizes[sizes.len() - 1], "it shrinks: {sizes:?}");
        assert!(sizes[0] > 0);
    }

    #[test]
    fn a_fading_ball_starts_bright_and_ends_dark() {
        let mut f = with(vec![ball(60.0, DEEP, 0.0, 0.0)]);
        let colour_after = |f: &mut Fireballs, frames: u32| {
            for _ in 0..frames {
                f.step(0.0);
            }
            let mut display = Display::new(COLS, ROWS);
            f.draw(&mut display);
            (0..ROWS as isize)
                .flat_map(|y| (0..COLS as isize).map(move |x| (x, y)))
                .find_map(|(x, y)| {
                    let pixel = display.pixel(x, y).unwrap();
                    (pixel.character() != ' ').then(|| pixel.colour())
                })
                .unwrap()
        };
        assert_eq!(colour_after(&mut f, 1), Colour::BrightWhite);
        // just before it shrinks away to nothing
        assert_eq!(colour_after(&mut f, 31), Colour::BrightBlack);
    }

    #[test]
    fn when_it_has_faded_a_new_ball_appears_above_the_water_going_its_own_way() {
        let mut f = with(vec![ball(60.0, DEEP, 0.3, 0.0), ball(20.0, 3.0, 0.1, 0.0)]);
        for _ in 0..FADE_FRAMES + 1 {
            f.step(0.0);
        }
        assert_eq!(f.respawns, 1);
        assert_eq!(f.balls.len(), 2, "still the same number of balls");
        let new = &f.balls[0];
        assert_eq!(new.fading, None);
        assert!(!in_water(&f, new, 0.0));
        assert!((new.vx, new.vy) != (0.3, 0.0), "a new velocity");
    }

    #[test]
    fn new_balls_appear_in_different_places() {
        let mut f = fireballs(9);
        f.step(0.0);
        let places: std::collections::BTreeSet<u32> =
            (0..50).map(|_| f.spawn(0.0).x.to_bits()).collect();
        assert!(places.len() > 40);
    }

    #[test]
    fn the_water_is_where_the_ocean_says_it_is_at_every_moment() {
        let mut f = fireballs(5);
        f.step(0.0);
        for step in 0..200 {
            let t = step as f32 * 0.37;
            let ball = f.spawn(t);
            assert!(!in_water(&f, &ball, t), "t={t}");
        }
    }

    // ---------------------------------------------------------------- safety

    #[test]
    fn a_long_run_keeps_four_balls_inside_the_window_and_replaces_the_drowned() {
        let mut f = fireballs(7);
        let (left, right, top, bottom) = f.bounds();
        for frame in 0..30 * 120 {
            f.step(frame as f32 / 30.0);
            assert_eq!(f.balls.len(), BALLS);
            for ball in &f.balls {
                assert!(ball.x >= left && ball.x <= right, "{}", ball.x);
                assert!(ball.y >= top && ball.y <= bottom, "{}", ball.y);
                assert!(ball.vx.is_finite() && ball.vy.is_finite());
            }
        }
        assert!(
            f.respawns > 3,
            "balls drown and are replaced: {}",
            f.respawns
        );
    }

    #[test]
    fn balls_mostly_bounce_around_before_they_fall_in() {
        // how long a ball lives, on average, over two simulated minutes
        let mut f = fireballs(11);
        for frame in 0..30 * 120 {
            f.step(frame as f32 / 30.0);
        }
        let seconds_each = 120.0 * BALLS as f32 / f.respawns.max(1) as f32;
        assert!(
            seconds_each > 4.0,
            "balls last {seconds_each:.1} s each, too short"
        );
    }

    #[test]
    fn it_survives_tiny_and_huge_windows_and_nonsense_numbers() {
        for (cols, rows) in [(1, 1), (2, 2), (5, 3), (1000, 1000)] {
            let mut f = Fireballs::new(cols, rows, 1);
            f.resize(cols, rows);
            for frame in 0..100 {
                f.step(frame as f32 / 30.0);
                let mut display = Display::new(cols, rows);
                f.draw(&mut display);
            }
        }
        let mut f = with(vec![
            ball(f32::NAN, 3.0, 0.0, 0.0),
            ball(10.0, f32::INFINITY, 0.0, 0.0),
        ]);
        f.step(0.0);
        assert!(f.respawns >= 1, "the NaN ball is replaced");
        let (left, right, top, bottom) = f.bounds();
        for ball in &f.balls {
            assert!(
                ball.x.is_finite() && ball.y.is_finite(),
                "nothing bad survives"
            );
            assert!(ball.x >= left && ball.x <= right && ball.y >= top && ball.y <= bottom);
        }
    }

    #[test]
    fn a_resize_keeps_the_balls_and_clears_the_trails() {
        let mut f = fireballs(1);
        f.step(0.0);
        f.resize(60, 20);
        assert_eq!(f.balls.len(), BALLS);
        assert_eq!(f.tails.len(), 60 * 20);
        f.step(0.1);
    }

    #[test]
    fn it_runs_as_a_layer() {
        let mut layers = Layers::new(COLS, ROWS, 2);
        layers.add::<Fireballs>();
        let mut m: Machine<Layers> = Machine::from_art(layers, COLS, ROWS);
        for _ in 0..30 {
            m.tick(34.0);
        }
        assert!(m.cells.chunks(2).any(|c| c[0] > 32), "something is drawn");
    }
}
