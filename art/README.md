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
3. In `fn all` in the same file add `layers.add::<comet::Comet>(cols, rows, seed);`.

Save. The new animation is on the page next to the sparkles. To turn one off, comment out
its `layers.add` line. To remove it, delete its file and its two lines.

Each animation keeps its own state and cannot see the others. To make two interact, put
them in one file.

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
