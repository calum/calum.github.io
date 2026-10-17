//! Runs a [`World`] (the library's ready-made balls, gravity and trails) as a
//! layer, so a few chained settings make an animation with no code.

use crate::{paint_over, Art, Display, Frame, Pointer, Simulation, World};

/// A [`World`] as an [`Art`]. Make one with `Layers::world`.
pub struct WorldArt {
    simulation: Simulation,
    /// The world draws every cell, blank ones too, so it draws here first and
    /// only the cells with something in them are copied onto the page.
    scratch: Display,
    clicks: bool,
}

impl WorldArt {
    /// A world for a window of `cols` by `rows` cells.
    pub fn new(world: &World, cols: u32, rows: u32, seed: u32) -> WorldArt {
        let mut simulation = world.simulation(seed);
        simulation.resize(cols, rows);
        WorldArt {
            simulation,
            scratch: Display::new(cols, rows),
            clicks: false,
        }
    }

    /// Clicks add a ball. Off by default: a click on the page is usually
    /// meant for the page.
    pub fn clickable(mut self) -> WorldArt {
        self.clicks = true;
        self
    }
}

impl Art for WorldArt {
    /// A plain `World::new()`. Use `Layers::world` to choose the settings.
    fn new(cols: u32, rows: u32, seed: u32) -> WorldArt {
        WorldArt::new(&World::new(), cols, rows, seed)
    }

    fn resize(&mut self, cols: u32, rows: u32) {
        self.simulation.resize(cols, rows);
        self.scratch = Display::new(cols, rows);
    }

    fn frame(&mut self, display: &mut Display, frame: &Frame) {
        // the world takes one step per frame, which is how its speeds are tuned
        if self.clicks {
            self.simulation.step(frame.pointer);
        } else {
            let pointer = frame.pointer;
            self.simulation.step(&Pointer {
                clicks: Vec::new(),
                ..pointer.clone()
            });
        }
        self.simulation.render(&mut self.scratch);

        paint_over(&self.scratch, display);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Colour, Layers, Machine, Palette};

    fn lit(machine: &Machine<WorldArt>) -> usize {
        (0..machine.cols * machine.rows)
            .filter(|i| machine.cells[(*i * 2) as usize] > 32)
            .count()
    }

    fn machine(world: &World) -> Machine<WorldArt> {
        Machine::from_art(WorldArt::new(world, 60, 20, 3), 60, 20)
    }

    #[test]
    fn balls_are_drawn_and_the_rest_of_the_page_is_left_empty() {
        let mut m = machine(&World::new().balls(3).size(2.0).palette(Palette::mono()));
        m.tick(40.0);
        let lit = lit(&m);
        assert!(lit > 0, "the balls are drawn");
        assert!(lit < 60 * 20 / 4, "most of the window stays empty: {lit}");
    }

    #[test]
    fn a_click_does_not_add_a_ball_unless_asked() {
        let still = World::new().balls(0).size(2.0);
        let mut m = machine(&still);
        m.pointer(1, 30.0, 10.0);
        m.tick(40.0);
        assert_eq!(lit(&m), 0, "by default the world ignores clicks");

        let mut m = Machine::from_art(WorldArt::new(&still, 60, 20, 3).clickable(), 60, 20);
        m.pointer(1, 30.0, 10.0);
        m.tick(40.0);
        assert!(lit(&m) > 0, "a clickable world gets a ball");
    }

    #[test]
    fn a_world_drawn_after_another_layer_does_not_erase_it() {
        struct Mark;
        impl Art for Mark {
            fn new(_: u32, _: u32, _: u32) -> Mark {
                Mark
            }
            fn frame(&mut self, display: &mut Display, _: &Frame) {
                display.set_pixel(0, 0, '*', Colour::Red, Colour::Black);
            }
        }
        let mut layers = Layers::new(60, 20, 3);
        layers.add::<Mark>();
        // a world with its balls far from the top left corner
        layers.world(World::new().balls(0));
        let mut m: Machine<Layers> = Machine::from_art(layers, 60, 20);
        m.tick(40.0);
        assert_eq!(m.cells[0], '*' as u32);
    }

    #[test]
    fn trails_stay_on_screen_after_the_clear_that_happens_every_frame() {
        let still = World::new().balls(1).size(1.0).gravity(0.0);
        let with = still.clone().trails();
        let count = |world: &World| {
            let mut m = machine(world);
            for _ in 0..40 {
                m.tick(34.0);
            }
            lit(&m)
        };
        assert!(
            count(&with) > count(&still),
            "a trail leaves more cells lit"
        );
    }

    #[test]
    fn a_resized_window_still_draws() {
        let mut m = machine(&World::new().balls(4).size(2.0));
        m.resize(120, 40);
        for _ in 0..5 {
            m.tick(34.0);
        }
        assert!(lit(&m) > 0);
    }
}
