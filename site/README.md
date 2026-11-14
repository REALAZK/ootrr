# OOTRR Web (preview)

The full OOTRR page, running in the browser on the player's own computer. The player picks their extracted game folder, generates a seed and gets the mod as a zip to unzip into their mods folder. Nothing is uploaded and the site holds no game data.

## How it works

- `ootrr serve --stdio` is the same server as `ootrr serve`, but it answers HTTP requests on stdin and stdout instead of a socket. It is compiled to `wasm32-wasip1`. Each request runs a fresh instance with the request on stdin, so nothing has to wait between requests.
- `wasi.js` gives it a small virtual file system. The game folder is mounted read only at `/game/<name>`. The mod is built into an in-memory `/zip/mods` folder and `zip.js` packs it for download.
- Chrome and Edge open the game folder with the File System Access API. Folders are listed and files are read only when the build opens them, and the WASI calls wait on the browser through JSPI (`WebAssembly.Suspending`).
- Firefox and other browsers use a folder input. The browser lists the files once and the worker reads them with `FileReaderSync`, so the WASI calls stay synchronous and need no JSPI.
- `host.js` runs the server and turns calls into HTTP requests. `worker.js` runs it in a Web Worker.
- The page is the desktop page from `crates/web/assets`. `make.mjs` copies it and swaps a few modules from `site/web`: `api.js` sends requests to the worker, `picker.js` opens the browser folder picker, and `setup.js` and `result.js` replace the mods folder with the zip download. `make.mjs` stops if the desktop `setup.js` or `result.js` change, so the web copies are kept in step.
- The tracker is left out for now. It watches save files with a background thread.

In `crates/web` the changes are small: `stdio.rs` (the stdin and stdout loop, with the token from `OOTRR_STDIO_TOKEN`), jobs run inline when there are no threads, and the token mix skips the process id on WASI.

## Build and try it locally

```
rustup target add wasm32-wasip1
cargo build --release -p ootrr --target wasm32-wasip1
node site/make.mjs
python -m http.server -d site/dist 8000
```

Then open http://localhost:8000 in Chrome, Edge or Firefox.

## Checks

These need Node 24 or newer.

- `node site/test/parity.mjs --exe target/release/ootrr --wasm target/wasm32-wasip1/release/ootrr.wasm` generates every preset with a few seeds through the same API calls the page makes, and compares the seed files with `ootrr generate`. It needs no game files and runs in CI.
- `node site/test/api_build.mjs --mode files|handle --wasm <ootrr.wasm> --game <romfs> --out <dir>` validates the game, generates and builds twice through the web code path in the Firefox or the Chrome mode, and writes the mod and its zip, for a byte comparison with `ootrr build`.

## Deploy

`.github/workflows/pages.yml` builds, tests, audits and checks on every pull request. On a push to `main` it also publishes `site/dist` to GitHub Pages. Set Settings, Pages, Source to GitHub Actions once.
