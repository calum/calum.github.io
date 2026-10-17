# The art over the site

Terminal-style art, written in Rust with
[terminal_graphics_interactive](https://github.com/calum/terminal_graphics_interactive),
compiled to WebAssembly and drawn on a transparent canvas over the whole page.
Clicks, scrolling and typing still reach the page, and the art sees them too.

## Work on it with live reload

```sh
rustup target add wasm32-unknown-unknown   # once
cd art
cargo dev                                  # then open http://localhost:8000
```

Save any file in `art/src` and the page rebuilds and reloads itself, usually in about
a second. If the code does not compile, the compiler's message appears in a red banner on
the page and the last working art keeps running. `cargo dev 9000` picks another port.
It also reloads when you edit `index.html` or `overlay.js`.

`cargo dev` writes `../art.wasm`. **Commit that file with your art**: GitHub Pages serves
it as is and does no build. (`./build.sh` rebuilds it without the server.)

## Add another animation (they stack, nothing is replaced)

Every animation is one file in `src/arts/`, and they all run at once, drawn in the order
they are listed, so later ones paint over earlier ones.

1. Copy `src/arts/sparkle.rs` to `src/arts/comet.rs`. Rename the struct inside (`Sparkle` to `Comet`)
   and change what it does. There is also a tiny template at the top of `src/arts/mod.rs`.
2. In `src/arts/mod.rs` add `mod comet;` next to `mod sparkle;`.
3. In `fn all` in the same file add `layers.add::<comet::Comet>();`.

Save. The new animation is on the page next to the sparkles. To turn one off, comment out
its `layers.add` line. To remove it, delete its file and its two lines.

Each animation keeps its own state and cannot see the others. To make two interact, put
them in one file.

## Or just paste a `World`, no file needed

The library's ready-made worlds (see its `examples/`) are one call in `fn all` in
`src/arts/mod.rs`. The bubbles on the site are exactly this:

```rust
layers.world(
    World::new()
        .balls(4)
        .size(3.0)
        .gravity(-0.004)
        .trails()
        .palette(Palette::mono()),
);
```

Take any `World::new()...` chain from the library, drop the `.run()` and pass it to
`layers.world(...)`. Add `.mouse_pulls()` if you want the mouse to attract the balls.
Clicks are left for the page and do not add balls; to let them, use `layers.world_clickable(...)`. `Palette::mono()`, `ocean()`, `fire()` and `rainbow()` exist.
Only the character and its foreground colour are painted, so the darkest shade of a
palette (a blank cell) is left see-through.

## Or paste a sketch (one formula for every cell)

Same idea for the library's `sketch_with` art: drop the `fn main` and the `sketch_with`,
and pass the palette and the formula to `layers.sketch(...)` in `fn all`.
The ocean on the site is one:

```rust
layers.sketch(Palette::ocean(), |cell| {
    let swell = (cell.x * 0.08 + cell.t).sin() * 0.5 + (cell.x * 0.19 - cell.t * 1.3).sin() * 0.25;
    let surface = cell.height / 2.0 + swell * 6.0;
    let glow = (-(cell.dist_to_mouse() * 0.12)).exp();
    ((surface - cell.y) * 0.25 + glow * 0.5).clamp(-1.0, 1.0)
});
```

**Mind the page.** A sketch gives every cell a shade, and in a terminal that fills the
screen. Here the page is underneath, so the darkest shade of the palette is see-through
but every other shade is painted. A formula that is bright over a large area covers the page
(the plain ocean above paints the whole top of the page solid white). Make the parts you want
to see through return `-1.0`, as the ocean in `src/arts/mod.rs` does for its brightest shades.

Layers are drawn in the order they are listed, so a layer later in the list paints over
the ones before it.

## What is on the site now

Drawn in this order (see `fn all` in `src/arts/mod.rs`): the sparkles, then the ocean, then the fireballs.
The fireballs go last so that you can see them sink into the water.

- **Ocean:** `src/arts/ocean.rs` says where the water is (`LEVEL`, how far down the window, and the waves).
  Both the ocean and the fireballs ask it, so they always agree. If you change `LEVEL`, change
  `--wave-clearance` in `index.html` to match.
- **Fireballs:** `src/arts/fireballs.rs`. They bounce off the walls and each other and are pulled down
  a tiny bit (`GRAVITY`, 0.001 per frame; 0 is none, the library's bubbles used -0.004 to rise). A ball
  that touches the waves (the bottom edge of the ball reaching the surface) turns grey, can no longer bounce off the walls or other balls, shrinks away in about 1.5 s, and a new ball appears
  somewhere above the water with a random velocity. `BALLS`, `RADIUS` and the fade length are consts at
  the top of the file. With the default gravity a ball lasts about 7 s on average.
  This is the library's `World` physics with the water added; a `World` cannot recolour or replace a ball.

## What an animation gets each frame

`fn frame(&mut self, display: &mut Display, frame: &Frame)` runs about 30 times a second.

| In `frame` | Meaning |
|---|---|
| `frame.cols`, `frame.rows` | window size in cells (changes on resize; implement `resize` to react) |
| `frame.seconds`, `frame.dt` | animation time, and seconds since the last frame |
| `frame.pointer` | `x`, `y` (cells, fractions), `left`, `right`, `clicks` since last frame; touch counts as a pointer |
| `frame.keys` | keys pressed since last frame |
| `frame.wheel` | wheel/trackpad rows since last frame, down is positive |
| `frame.scroll` | how far down the page is scrolled, 0.0 to 1.0 |

`display` starts empty each frame; empty cells show the page underneath. Draw with
`display.set_pixel(x, y, char, colour, background)`. Only the character and its colour are
painted, so use `'█'` for solid blocks. `Rng::new(seed)` is a small seeded random generator
(`next_f32()`, `range(low, high)`).

The cell size is `FONT_PX` at the top of `../overlay.js`. People with "reduce motion"
turned on get no overlay.

## Tests

`cargo test` checks the plumbing, the dev server and the template in `src/arts/mod.rs` on your machine.
