//! Runs a sketch (one formula for every cell, like the library's `sketch_with`)
//! as a layer, so a formula makes an animation with no other code.

use crate::{paint_over, Art, Cell, Display, Frame, Palette};

/// A sketch as an [`Art`]. Make one with `Layers::sketch`.
pub struct SketchArt {
    palette: Palette,
    formula: Box<dyn FnMut(&Cell) -> f32>,
    /// The sketch fills every cell, so it draws here first and only the cells
    /// with something in them are copied onto the page.
    scratch: Display,
}

impl SketchArt {
    /// A sketch for a window of `cols` by `rows` cells. The formula gets every
    /// cell and returns -1.0 to 1.0, which `palette` turns into a shade.
    pub fn new(
        palette: Palette,
        cols: u32,
        rows: u32,
        formula: impl FnMut(&Cell) -> f32 + 'static,
    ) -> SketchArt {
        SketchArt {
            palette,
            formula: Box::new(formula),
            scratch: Display::new(cols, rows),
        }
    }
}

impl Art for SketchArt {
    /// A sketch that draws nothing. Use `Layers::sketch` to give it a formula.
    fn new(cols: u32, rows: u32, _seed: u32) -> SketchArt {
        SketchArt::new(Palette::mono(), cols, rows, |_| -1.0)
    }

    fn resize(&mut self, cols: u32, rows: u32) {
        self.scratch = Display::new(cols, rows);
    }

    fn frame(&mut self, display: &mut Display, frame: &Frame) {
        let formula = &mut self.formula;
        crate::sketch_frame(
            &mut self.scratch,
            &self.palette,
            frame.seconds,
            frame.pointer,
            |cell| formula(cell),
        );
        paint_over(&self.scratch, display);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Layers, Machine};

    fn lit(machine: &Machine<Layers>) -> usize {
        machine.cells.chunks(2).filter(|c| c[0] > 32).count()
    }

    fn machine(formula: impl FnMut(&Cell) -> f32 + 'static) -> Machine<Layers> {
        let mut layers = Layers::new(40, 20, 1);
        layers.sketch(Palette::ocean(), formula);
        Machine::from_art(layers, 40, 20)
    }

    #[test]
    fn cells_the_formula_leaves_at_the_bottom_of_the_range_stay_see_through() {
        let mut m = machine(|_| -1.0);
        m.tick(40.0);
        assert_eq!(lit(&m), 0);
    }

    #[test]
    fn a_formula_paints_only_where_it_is_bright() {
        // the left half is bright, the right half is dark
        let mut m = machine(|cell| if cell.x < 20.0 { 1.0 } else { -1.0 });
        m.tick(40.0);
        assert_eq!(lit(&m), 20 * 20);
    }

    #[test]
    fn the_formula_sees_time_and_the_pointer() {
        let mut m = machine(|cell| {
            if cell.t > 0.5 && cell.dist_to_mouse() < 3.0 {
                1.0
            } else {
                -1.0
            }
        });
        m.pointer(0, 20.0, 10.0);
        m.tick(40.0);
        assert_eq!(lit(&m), 0, "too early");
        for _ in 0..20 {
            m.tick(34.0);
        }
        assert!(lit(&m) > 0, "later, near the mouse");
    }

    #[test]
    fn a_sketch_drawn_after_another_layer_does_not_erase_it() {
        let mut layers = Layers::new(40, 20, 1);
        layers.world(crate::World::new().balls(0));
        layers.sketch(Palette::mono(), |_| -1.0);
        let mut m: Machine<Layers> = Machine::from_art(layers, 40, 20);
        m.pointer(1, 5.0, 5.0);
        m.tick(40.0);
        assert_eq!(lit(&m), 0);
    }

    #[test]
    fn a_resized_window_still_draws() {
        let mut m = machine(|_| 1.0);
        m.resize(80, 30);
        m.tick(40.0);
        assert_eq!(lit(&m), 80 * 30);
    }
}
