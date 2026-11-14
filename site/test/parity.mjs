import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Host } from "../host.js";
import { effective } from "../../crates/web/assets/js/rules.js";

const args = process.argv.slice(2);
const flag = (k) => {
  const i = args.indexOf(k);
  if (i < 0) throw new Error("usage: parity.mjs --exe <native ootrr> --wasm <ootrr.wasm>");
  return args[i + 1];
};
const exe = flag("--exe");
const h = new Host(new WebAssembly.Module(readFileSync(flag("--wasm"))), { async: args.includes("--async") });

async function call(method, path, body) {
  const r = await h.request(method, path, body === undefined ? null : JSON.stringify(body));
  return { status: r.status, data: JSON.parse(new TextDecoder().decode(r.body)) };
}

const schema = (await call("GET", "/api/schema")).data;
const seeds = ["1", "12345", "Kokiri Forest"];
const dir = mkdtempSync(join(tmpdir(), "ootrr-parity-"));
let bad = 0;
try {
  for (const p of schema.presets) {
    for (const s of seeds) {
      const seed_p = join(dir, "seed.ootrr");
      rmSync(seed_p, { force: true });
      const n = spawnSync(exe, ["generate", p.id, s, seed_p], { encoding: "utf8" });
      const r = await call("POST", "/api/generate", { seed: s, settings: effective(schema, p.settings), spoiler: true });
      let same;
      let how;
      if (n.status === 0) {
        same = r.status === 200 && r.data.seed_text === readFileSync(seed_p, "utf8");
        how = "same seed file";
      } else {
        const why = n.stderr.trim().split(/\r?\n/).slice(1).map((x) => x.trim());
        same = r.status >= 400 && why.every((x) => r.data.error.includes(x));
        how = "refused the same way";
      }
      if (!same) bad++;
      console.log(`${same ? "same" : "DIFFERENT"}  ${p.id} / ${s}  (${how})`);
    }
  }
} finally {
  rmSync(dir, { recursive: true, force: true });
}
if (bad) {
  console.error(`${bad} seeds differ between the native build and the web build`);
  process.exit(1);
}
console.log("the web build generates the same seeds as the native build, through the same API the page uses");
process.exit(0);
