// Runs the terminal art in art.wasm over the whole page (see art/ for the Rust).
//
// The canvas ignores the pointer (pointer-events: none) and every listener here
// is passive and never calls preventDefault, so clicks, scrolling and typing
// reach the page exactly as if the art were not there, and the art sees them too.
(function () {
  "use strict";

  var FONT_PX = 16;
  var FONT = FONT_PX + 'px "Courier New", Courier, monospace';

  // Respect people who ask for less motion: no overlay at all.
  if (window.matchMedia && matchMedia("(prefers-reduced-motion: reduce)").matches) return;

  var canvas = document.getElementById("art");
  if (!canvas || !window.WebAssembly) return;
  var ctx = canvas.getContext("2d");

  var wasm, memory;
  var cellW, cellH, dpr, cols, rows;
  var palette = [];
  var last = 0;
  var running = true;

  // Named keys travel as 0x110000 + n; the same table is in art/src/lib.rs.
  var NAMED = 0x110000;
  var KEYS = {
    Enter: 1, Tab: 2, Backspace: 3, Escape: 4, ArrowUp: 5, ArrowDown: 6,
    ArrowLeft: 7, ArrowRight: 8, Home: 9, End: 10, PageUp: 11, PageDown: 12,
    Insert: 13, Delete: 14,
  };
  for (var f = 1; f <= 12; f++) KEYS["F" + f] = 20 + f;

  function stop(error) {
    running = false;
    canvas.remove();
    if (error) console.error("art stopped:", error);
  }

  function measure() {
    ctx.font = FONT;
    cellW = ctx.measureText("M").width;
    cellH = Math.round(FONT_PX * 1.2);
  }

  // Sizes the canvas to the window and the grid to the canvas.
  function fit() {
    dpr = window.devicePixelRatio || 1;
    var w = document.documentElement.clientWidth;
    var h = document.documentElement.clientHeight;
    canvas.style.width = w + "px";
    canvas.style.height = h + "px";
    canvas.width = Math.round(w * dpr);
    canvas.height = Math.round(h * dpr);
    cols = Math.max(1, Math.ceil(w / cellW));
    rows = Math.max(1, Math.ceil(h / cellH));
    wasm.resize(cols, rows);
    sendScroll();
  }

  function sendScroll() {
    var root = document.documentElement;
    var range = root.scrollHeight - root.clientHeight;
    wasm.scroll(range > 0 ? window.scrollY / range : 0);
  }

  function paint() {
    var width = wasm.width(), height = wasm.height();
    var cells = new Uint32Array(memory.buffer, wasm.frame_ptr(), width * height * 2);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, canvas.width / dpr, canvas.height / dpr);
    ctx.font = FONT;
    ctx.textBaseline = "middle";
    var current = -1;
    for (var y = 0; y < height; y++) {
      for (var x = 0; x < width; x++) {
        var i = (y * width + x) * 2;
        var ch = cells[i];
        if (ch <= 32) continue; // a blank cell shows the page underneath
        var colour = cells[i + 1] & 255;
        if (colour !== current) {
          ctx.fillStyle = palette[colour];
          current = colour;
        }
        if (ch === 0x2588) {
          ctx.fillRect(x * cellW, y * cellH, Math.ceil(cellW), cellH); // solid block
        } else {
          ctx.fillText(String.fromCodePoint(ch), x * cellW, y * cellH + cellH / 2);
        }
      }
    }
  }

  function frame(now) {
    if (!running) return;
    try {
      if (wasm.tick(last ? now - last : 0)) paint();
    } catch (error) {
      return stop(error);
    }
    last = now;
    requestAnimationFrame(frame);
  }

  // ---- input, forwarded to the art and left alone for the page

  function forward(kind, event) {
    wasm.pointer(kind, event.clientX / cellW, event.clientY / cellH);
  }
  window.addEventListener("pointermove", function (e) {
    forward(e.buttons & 1 ? 5 : e.buttons & 2 ? 6 : 0, e);
  }, { passive: true });
  window.addEventListener("pointerdown", function (e) {
    forward(e.button === 2 ? 3 : e.button === 0 ? 1 : 0, e);
  }, { passive: true });
  function release(e) { forward(e.button === 2 ? 4 : 2, e); }
  window.addEventListener("pointerup", release, { passive: true });
  window.addEventListener("pointercancel", release, { passive: true });

  window.addEventListener("wheel", function (e) {
    // deltaMode: 0 pixels, 1 lines, 2 pages
    var rowsMoved = e.deltaMode === 0 ? e.deltaY / cellH : e.deltaMode === 1 ? e.deltaY : e.deltaY * rows;
    wasm.wheel(rowsMoved);
  }, { passive: true });
  window.addEventListener("scroll", sendScroll, { passive: true });

  window.addEventListener("keydown", function (e) {
    var code = KEYS[e.key];
    if (code !== undefined) code += NAMED;
    else if (e.key.length === 1 || [...e.key].length === 1) code = e.key.codePointAt(0);
    else return; // Shift, Control, Dead keys and the like
    wasm.key(code, (e.shiftKey ? 1 : 0) | (e.altKey ? 2 : 0) | (e.ctrlKey || e.metaKey ? 4 : 0));
  }, { passive: true });

  var resizeTimer;
  window.addEventListener("resize", function () {
    clearTimeout(resizeTimer);
    resizeTimer = setTimeout(fit, 100);
  });

  fetch("art.wasm")
    .then(function (response) { return response.arrayBuffer(); })
    .then(function (bytes) { return WebAssembly.instantiate(bytes, {}); })
    .then(function (result) {
      wasm = result.instance.exports;
      memory = wasm.memory;
      for (var i = 0; i < 16; i++) {
        palette.push("#" + (wasm.palette(i) | 0x1000000).toString(16).slice(1));
      }
      measure();
      wasm.init(1, 1, (Math.random() * 0xffffffff) >>> 0);
      fit();
      requestAnimationFrame(frame);
    })
    .catch(stop);
})();
