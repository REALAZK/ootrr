import { readFileSync, readdirSync, statSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { Host, MODS } from "../host.js";
import { zip } from "../zip.js";
import { DirHandle } from "./nodefs.mjs";

const args = process.argv.slice(2);
const flag = (k, d) => {
  const i = args.indexOf(k);
  if (i >= 0) return args[i + 1];
  if (d === undefined) throw new Error("usage: api_build.mjs --wasm <ootrr.wasm> --game <romfs> --out <dir> [--mode files|handle] [--host hookshot] [--seed 12345] [--name randomizer-12345]");
  return d;
};
const mode = flag("--mode", "files");
const host_kind = flag("--host", "hookshot");
const seed = flag("--seed", "12345");
const name = flag("--name", `randomizer-${seed}`);
const game = flag("--game");
const out = flag("--out");

function scan(root) {
  const found = [];
  const stack = [""];
  while (stack.length) {
    const rel = stack.pop();
    for (const e of readdirSync(join(root, rel), { withFileTypes: true })) {
      const r = rel ? `${rel}/${e.name}` : e.name;
      if (e.isDirectory()) stack.push(r);
      else found.push([r, { path: join(root, r), size: statSync(join(root, r)).size }]);
    }
  }
  return found;
}

const h = new Host(new WebAssembly.Module(readFileSync(flag("--wasm"))), {
  async: mode === "handle",
  reader: (src) => {
    const b = readFileSync(src.path);
    return new Uint8Array(b.buffer, b.byteOffset, b.length);
  },
  log: (t) => process.stderr.write(t),
});

async function call(method, path, body) {
  const t = performance.now();
  const r = await h.request(method, path, body === undefined ? null : JSON.stringify(body));
  const text = new TextDecoder().decode(r.body);
  console.error(`${method} ${path} -> ${r.status} in ${Math.round(performance.now() - t)} ms`);
  if (r.status < 200 || r.status > 299) throw new Error(`${path}: ${text}`);
  return { data: JSON.parse(text), built: r.built };
}

let t = performance.now();
if (mode === "handle") await h.mount_handle("/game/romfs", new DirHandle(game));
else await h.mount_files("/game/romfs", scan(game));
console.error(`${mode}: game mounted in ${Math.round(performance.now() - t)} ms`);

await call("GET", "/api/info");
await call("GET", "/api/schema");
const v = (await call("GET", `/api/validate?path=${encodeURIComponent("/game/romfs")}`)).data;
if (!v.ok) throw new Error(`validate: ${v.problems.join(" ")}`);
const g = (await call("POST", "/api/generate", { seed, settings: {}, spoiler: true })).data;
mkdirSync(out, { recursive: true });
writeFileSync(join(out, "seed.ootrr"), g.seed_text);
let files = null;
for (let round = 1; round <= 2; round++) {
  const b = (await call("POST", "/api/build", { seed_text: g.seed_text, game_dir: "/game/romfs", mod_dir: MODS, mod_name: name, host: host_kind, music: "off", cosmetic_seed: "" })).data;
  const job = await call("GET", `/api/jobs/${b.job_id}`);
  if (job.data.state !== "done") throw new Error(job.data.message);
  files = job.built;
  console.error(`round ${round}: ${job.data.state}, ${files.filter(([, x]) => x).length} files`);
}
for (const [p, b] of files) {
  const to = join(out, "mods", p);
  if (b === null) mkdirSync(to, { recursive: true });
  else {
    mkdirSync(dirname(to), { recursive: true });
    writeFileSync(to, b);
  }
}
writeFileSync(join(out, "mod.zip"), Buffer.from(await zip(files).arrayBuffer()));
console.error("ok");
process.exit(0);
