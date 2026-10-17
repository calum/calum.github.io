# The art over the site

Terminal-style art, written in Rust with
[terminal_graphics_interactive](https://github.com/calum/terminal_graphics_interactive),
compiled to WebAssembly and drawn on a transparent canvas over the whole page.
Clicks, scrolling and typing still reach the page, and the art sees them too.

## Make your art

Edit **`src/my_art.rs`** (the starter there is a placeholder, replace all of it), then:

```sh
rustup target add wasm32-unknown-unknown   # once
./build.sh                                 # writes ../art.wasm
python3 -m http.server -d ..               # try it at http://localhost:8000
```

`art.wasm` is committed, so GitHub Pages needs no build step: commit it with your art.
`cargo test` runs the plumbing tests on your machine.

## What your art gets each frame

`Art::frame(&mut self, display, frame)` runs about 30 times a second.

| In `frame` | Meaning |
|---|---|
| `frame.cols`, `frame.rows` | window size in cells (changes on resize; override `resize` to react) |
| `frame.seconds`, `frame.dt` | animation time, and seconds since the last frame |
| `frame.pointer` | `x`, `y` (cells, fractions), `left`, `right`, `clicks` since last frame; touch counts as a pointer |
| `frame.keys` | keys pressed since last frame |
| `frame.wheel` | wheel/trackpad rows since last frame, down is positive |
| `frame.scroll` | how far down the page is scrolled, 0.0 to 1.0 |

`display` starts empty each frame; empty cells show the page underneath. Draw with
`display.set_pixel(x, y, char, colour, background)`. Only the character and its colour are
painted, so use `'█'` for solid blocks. `Rng` is a small seeded random generator.

Settings you may want to change are the top of `../overlay.js` (`FONT_PX`, the cell size).
People with "reduce motion" turned on get no overlay.
