//! `cargo dev`: a live-reload server for working on the art.
//!
//! It serves the site folder at http://localhost:8000, rebuilds `art.wasm`
//! whenever a file under `art/src` changes, and reloads the browser when the
//! build succeeds (or when `index.html` or `overlay.js` change). If the build
//! fails, the compiler errors appear in a red banner on the page and the last
//! good art keeps running. Only for your machine: nothing here is published.
//!
//! `cargo dev 9000` uses another port. Stop it with Ctrl+C.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

/// Added to every page this server sends, so the browser asks "anything new?".
const RELOAD_SNIPPET: &str = r#"<script>
(function () {
  var version = null, banner = null;
  function show(text) {
    if (!text) { if (banner) { banner.remove(); banner = null; } return; }
    if (!banner) {
      banner = document.createElement("pre");
      banner.style.cssText = "position:fixed;left:0;right:0;top:0;max-height:50vh;overflow:auto;margin:0;padding:12px;background:#a00;color:#fff;font:13px monospace;z-index:99999;white-space:pre-wrap;text-align:left";
      document.body.appendChild(banner);
    }
    banner.textContent = text;
  }
  setInterval(function () {
    fetch("/__reload", { cache: "no-store" }).then(function (r) { return r.text(); }).then(function (text) {
      var line = text.indexOf("\n"), v = text.slice(0, line), error = text.slice(line + 1);
      show(error);
      if (version !== null && v !== version && !error) location.reload();
      version = v;
    }).catch(function () {});
  }, 400);
})();
</script>
"#;

/// What the browser polls for: a number that goes up on every good change,
/// and the compiler output if the last build failed.
#[derive(Default)]
struct State {
    version: u64,
    error: String,
}

fn main() {
    let port: u16 = std::env::args()
        .nth(1)
        .map(|arg| {
            arg.parse()
                .unwrap_or_else(|_| fail("the port must be a number, like `cargo dev 9000`"))
        })
        .unwrap_or(8000);

    let art_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let site_dir = art_dir.join("..");
    let state = Arc::new(Mutex::new(State::default()));

    let listener = TcpListener::bind(("127.0.0.1", port)).unwrap_or_else(|e| {
        fail(&format!(
            "cannot use port {port}: {e}. Try `cargo dev 9000`"
        ))
    });

    build(&art_dir, &site_dir, &state);
    println!("Serving the site at http://localhost:{port}  (Ctrl+C to stop)");
    println!("Edit anything in art/src and save; the page reloads when it builds.");

    {
        let state = Arc::clone(&state);
        let (art_dir, site_dir) = (art_dir.clone(), site_dir.clone());
        thread::spawn(move || watch(&art_dir, &site_dir, &state));
    }

    for stream in listener.incoming().flatten() {
        let state = Arc::clone(&state);
        let site_dir = site_dir.clone();
        thread::spawn(move || {
            let _ = serve(stream, &site_dir, &state);
        });
    }
}

fn fail(message: &str) -> ! {
    eprintln!("{message}");
    std::process::exit(1)
}

// ------------------------------------------------------------------ building

/// Compiles the art to `../art.wasm`. A failure is kept for the page to show.
fn build(art_dir: &Path, site_dir: &Path, state: &Mutex<State>) {
    println!("Building...");
    let output = Command::new("cargo")
        .args([
            "build",
            "--release",
            "--lib",
            "--target",
            "wasm32-unknown-unknown",
            "--color",
            "never",
        ])
        .current_dir(art_dir)
        .output();
    let mut state = state.lock().unwrap();
    let output = match output {
        Ok(output) => output,
        Err(e) => {
            state.error = format!("could not run cargo: {e}");
            return;
        }
    };
    if !output.status.success() {
        state.error = String::from_utf8_lossy(&output.stderr).into_owned();
        eprintln!("{}", state.error);
        println!("Build failed (the error is shown on the page).");
        return;
    }
    let built = art_dir.join("target/wasm32-unknown-unknown/release/site_art.wasm");
    match fs::copy(built, site_dir.join("art.wasm")) {
        Ok(_) => {
            state.error.clear();
            state.version += 1;
            println!("Built art.wasm, reloading the page.");
        }
        Err(e) => state.error = format!("could not write art.wasm: {e}"),
    }
}

/// Watches for saves, a few times a second. Plain polling of file times keeps
/// this free of dependencies and the same on Windows, macOS and Linux.
fn watch(art_dir: &Path, site_dir: &Path, state: &Mutex<State>) {
    let rust_files = |list: &mut Vec<PathBuf>| {
        collect(&art_dir.join("src"), list);
        list.push(art_dir.join("Cargo.toml"));
    };
    let page_files = |list: &mut Vec<PathBuf>| {
        list.push(site_dir.join("index.html"));
        list.push(site_dir.join("overlay.js"));
    };
    let mut rust_seen = newest(rust_files);
    let mut page_seen = newest(page_files);
    loop {
        thread::sleep(Duration::from_millis(300));
        let rust_now = newest(rust_files);
        if rust_now != rust_seen {
            rust_seen = rust_now;
            build(art_dir, site_dir, state);
        }
        let page_now = newest(page_files);
        if page_now != page_seen {
            page_seen = page_now;
            state.lock().unwrap().version += 1;
        }
    }
}

/// The most recent modification time of the files `list` names.
fn newest(list: impl Fn(&mut Vec<PathBuf>)) -> Option<SystemTime> {
    let mut files = Vec::new();
    list(&mut files);
    files
        .iter()
        .filter_map(|file| fs::metadata(file).ok()?.modified().ok())
        .max()
}

/// Every file below `dir`.
fn collect(dir: &Path, list: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, list);
        } else {
            list.push(path);
        }
    }
}

// ------------------------------------------------------------------- serving

fn serve(mut stream: TcpStream, site_dir: &Path, state: &Mutex<State>) -> std::io::Result<()> {
    // Browsers open spare connections and send nothing on them. Give up on those
    // quickly, or they use up the browser's few connections and the page stalls.
    stream.set_read_timeout(Some(Duration::from_millis(300)))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    // the rest of the request (headers) is not needed
    let mut line = String::new();
    while reader.read_line(&mut line)? > 2 {
        line.clear();
    }

    let Some(url) = request_path(&request_line) else {
        return respond(&mut stream, "400 Bad Request", "text/plain", b"bad request");
    };

    if url == "/__reload" {
        let state = state.lock().unwrap();
        let body = format!("{}\n{}", state.version, state.error);
        return respond(
            &mut stream,
            "200 OK",
            "text/plain; charset=utf-8",
            body.as_bytes(),
        );
    }

    let Some(file) = resolve(site_dir, &url) else {
        return respond(&mut stream, "404 Not Found", "text/plain", b"not found");
    };
    let Ok(bytes) = fs::read(&file) else {
        return respond(&mut stream, "404 Not Found", "text/plain", b"not found");
    };
    let kind = content_type(&file);
    if kind.starts_with("text/html") {
        let page = inject(&String::from_utf8_lossy(&bytes));
        return respond(&mut stream, "200 OK", kind, page.as_bytes());
    }
    respond(&mut stream, "200 OK", kind, &bytes)
}

fn respond(stream: &mut TcpStream, status: &str, kind: &str, body: &[u8]) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)
}

/// `GET /index.html?x=1 HTTP/1.1` gives `/index.html`.
fn request_path(request_line: &str) -> Option<String> {
    let mut parts = request_line.split_whitespace();
    if parts.next()? != "GET" {
        return None;
    }
    let target = parts.next()?;
    Some(target.split(['?', '#']).next()?.to_string())
}

/// The file a URL means, or `None` if it would leave the site folder.
fn resolve(site_dir: &Path, url: &str) -> Option<PathBuf> {
    let relative = url.trim_start_matches('/');
    let relative = if relative.is_empty() {
        "index.html"
    } else {
        relative
    };
    let mut path = site_dir.to_path_buf();
    for part in relative.split('/') {
        if part.is_empty() || part == "." || part == ".." || part.contains(['\\', ':']) {
            return None;
        }
        path.push(part);
    }
    // never serve the Rust sources or build folder
    if relative.starts_with("art/") {
        return None;
    }
    Some(path)
}

fn content_type(file: &Path) -> &'static str {
    match file.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("wasm") => "application/wasm",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("ttf") => "font/ttf",
        Some("otf") => "font/otf",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    }
}

/// Puts the reload script at the end of the page.
fn inject(page: &str) -> String {
    match page.rfind("</body>") {
        Some(at) => format!("{}{}{}", &page[..at], RELOAD_SNIPPET, &page[at..]),
        None => format!("{page}{RELOAD_SNIPPET}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_path_is_taken_from_a_get_request() {
        assert_eq!(
            request_path("GET /a/b.js?x=1 HTTP/1.1\r\n").as_deref(),
            Some("/a/b.js")
        );
        assert_eq!(request_path("GET / HTTP/1.1").as_deref(), Some("/"));
        assert_eq!(request_path("POST / HTTP/1.1"), None);
        assert_eq!(request_path(""), None);
    }

    #[test]
    fn urls_cannot_leave_the_site_folder_or_reach_the_rust_sources() {
        let site = Path::new("/site");
        assert_eq!(resolve(site, "/"), Some(PathBuf::from("/site/index.html")));
        assert_eq!(
            resolve(site, "/art.wasm"),
            Some(PathBuf::from("/site/art.wasm"))
        );
        assert_eq!(
            resolve(site, "/fonts/Myfont-Regular.ttf"),
            Some(PathBuf::from("/site/fonts/Myfont-Regular.ttf"))
        );
        for bad in [
            "/../secret",
            "/a/../../b",
            "/..\\x",
            "/C:/x",
            "/art/src/lib.rs",
            "/art/Cargo.toml",
        ] {
            assert_eq!(resolve(site, bad), None, "{bad}");
        }
    }

    #[test]
    fn a_doubled_slash_stays_inside_the_site_folder() {
        assert_eq!(
            resolve(Path::new("/site"), "//etc/passwd"),
            Some(PathBuf::from("/site/etc/passwd"))
        );
    }

    #[test]
    fn the_reload_script_goes_just_before_the_end_of_the_body() {
        let page = inject("<html><body>hi</body></html>");
        let script = page.find("<script>").unwrap();
        assert!(page.find("hi").unwrap() < script && script < page.find("</body>").unwrap());
        assert!(inject("no body tag").ends_with(RELOAD_SNIPPET));
    }

    #[test]
    fn wasm_is_served_with_the_type_browsers_need_to_stream_it() {
        assert_eq!(content_type(Path::new("art.wasm")), "application/wasm");
        assert!(content_type(Path::new("index.html")).starts_with("text/html"));
        assert_eq!(
            content_type(Path::new("mystery")),
            "application/octet-stream"
        );
    }
}
