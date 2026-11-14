import { Host } from "./host.js";
import { zip } from "./zip.js";

const reader = typeof FileReaderSync === "function" ? new FileReaderSync() : null;
let made = null;

const host = (async () => {
  const r = await fetch(new URL("./ootrr.wasm", import.meta.url));
  if (!r.ok) throw new Error(`ootrr.wasm could not be loaded (${r.status})`);
  const m = r.headers.get("content-type") === "application/wasm"
    ? await WebAssembly.compileStreaming(r)
    : await WebAssembly.compile(await r.arrayBuffer());
  const jspi = typeof WebAssembly.Suspending === "function" && typeof WebAssembly.promising === "function";
  return new Host(m, {
    async: jspi,
    reader: (f) => new Uint8Array(reader.readAsArrayBuffer(f)),
    log: (t) => console.warn(t),
  });
})();

async function run(d) {
  const h = await host;
  if (d.op === "hello") return { jspi: h.async, sync_files: !!reader };
  if (d.op === "mount_handle") {
    if (!h.async) throw new Error("this browser cannot read a picked folder in place");
    await h.mount_handle(d.at, d.handle);
    return {};
  }
  if (d.op === "mount_files") {
    if (!reader) throw new Error("this browser cannot read files in a worker");
    await h.mount_files(d.at, d.entries);
    return {};
  }
  const r = await h.request(d.method, d.path, d.body);
  const out = { status: r.status, ctype: r.ctype, body: r.body };
  if (r.built) {
    if (!made || made.from !== r.built) made = { from: r.built, blob: zip(r.built) };
    out.zip = made.blob;
  }
  return out;
}

self.onmessage = (e) => {
  const d = e.data;
  run(d).then(
    (r) => self.postMessage({ id: d.id, ok: true, ...r }, r.body ? [r.body.buffer] : []),
    (err) => self.postMessage({ id: d.id, ok: false, message: String((err && err.message) || err) }),
  );
};
