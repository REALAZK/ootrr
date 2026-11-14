import { createHash } from "node:crypto";
import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, "..");
const assets = join(repo, "crates", "web", "assets");
const args = process.argv.slice(2);
const flag = (k, d) => {
  const i = args.indexOf(k);
  return i >= 0 ? args[i + 1] : d;
};
const wasm = flag("--wasm", join(repo, "target", "wasm32-wasip1", "release", "ootrr.wasm"));
const out = flag("--out", join(here, "dist"));
const SOURCE = "https://github.com/realazk/ootrr";

if (!existsSync(wasm)) {
  console.error(`no ${wasm}. Run: cargo build --release -p ootrr --target wasm32-wasip1`);
  process.exit(1);
}

function edit(p, f) {
  const before = readFileSync(p, "utf8");
  const after = f(before);
  if (after === before) throw new Error(`${p} did not change, the page source moved on and make.mjs needs an update`);
  writeFileSync(p, after);
}

function must(text, find, put) {
  if (!text.includes(find)) throw new Error(`expected to find ${find}`);
  return text.replaceAll(find, put);
}

rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });
cpSync(assets, out, { recursive: true, filter: (p) => !/[\\/]fonts$/.test(p) && !/tracker[^\\/]*\.(html|css|js)$/.test(p) });
mkdirSync(join(out, "fonts"));
const fonts = {
  "cinzel.woff2": "cinzel-latin-wght-normal.woff2",
  "garamond.woff2": "eb-garamond-latin-wght-normal.woff2",
  "garamond-italic.woff2": "eb-garamond-latin-wght-italic.woff2",
  "OFL-Cinzel.txt": "OFL-Cinzel.txt",
  "OFL-EBGaramond.txt": "OFL-EBGaramond.txt",
};
for (const [to, from] of Object.entries(fonts)) cpSync(join(assets, "fonts", from), join(out, "fonts", to));

const BASED_ON = { "setup.js": "7fefa5a916bc07a6", "result.js": "b59b790872997094" };
for (const [f, want] of Object.entries(BASED_ON)) {
  const got = createHash("sha256").update(readFileSync(join(assets, "js", f), "utf8").replace(/\r\n/g, "\n")).digest("hex").slice(0, 16);
  if (got !== want) throw new Error(`crates/web/assets/js/${f} changed since site/web/${f} was written from it. Carry the change over to site/web/${f}, then set its hash in make.mjs to ${got}.`);
}
for (const f of ["api.js", "picker.js", "folders.js", "setup.js", "result.js"]) cpSync(join(here, "web", f), join(out, "js", f));
const ROOT = ["worker.js", "host.js", "wasi.js", "zip.js"];
for (const f of ROOT) cpSync(join(here, f), join(out, f));
cpSync(wasm, join(out, "ootrr.wasm"));
for (const f of ["LICENSE", "NOTICE"]) cpSync(join(repo, f), join(out, `${f}.txt`));

for (const f of readdirSync(out).filter((x) => x.endsWith(".js") && !ROOT.includes(x))) {
  edit(join(out, f), (t) => t.replaceAll('"/js/', '"./js/'));
}
for (const f of readdirSync(join(out, "js")).filter((x) => x.endsWith(".js"))) {
  const p = join(out, "js", f);
  const t = readFileSync(p, "utf8");
  if (t.includes('"/js/')) writeFileSync(p, t.replaceAll('"/js/', '"./'));
}
for (const f of readdirSync(out).filter((x) => x.endsWith(".css"))) {
  const p = join(out, f);
  writeFileSync(p, readFileSync(p, "utf8").replaceAll('url("/fonts/', 'url("fonts/'));
}
edit(join(out, "index.html"), (t) => {
  let s = t.replace(/(href|src)="\/(?!\/)/g, '$1="');
  s = must(s, '<a class="btn small" href="tracker">Tracker</a>', `<a class="btn small" href="${SOURCE}" rel="noopener">Source</a>`);
  s = must(s, "<title>OOTRR</title>", '<title>OOTRR Web</title>\n<link rel="icon" href="data:,">');
  return s;
});

const left = [];
for (const f of readdirSync(out, { recursive: true }).filter((x) => /\.(js|html|css)$/.test(x))) {
  const t = readFileSync(join(out, f), "utf8");
  if (/["'(]\/(js|fonts)\//.test(t) || /(href|src)="\/[^/]/.test(t)) left.push(f);
}
if (left.length) throw new Error(`root paths left in ${left.join(", ")}`);
console.log(`site written to ${out}`);
