import { restore } from "./folders.js";

export class ApiError extends Error {
  constructor(status, message) {
    super(message);
    this.status = status;
  }
}

const NEEDS = "This page runs OOTRR on your own computer and needs a desktop browser with WebAssembly and module workers, such as a current Chrome, Edge or Firefox.";

let worker = null;
let seq = 0;
const waits = new Map();
let boot = null;
let zip = null;

export const MODS = "/zip/mods";

export function in_place() {
  return typeof window.showDirectoryPicker === "function" && typeof WebAssembly.Suspending === "function" && typeof WebAssembly.promising === "function";
}

export function last_zip() {
  return zip;
}

function call(d) {
  return new Promise((ok, bad) => {
    const id = ++seq;
    waits.set(id, { ok, bad });
    worker.postMessage({ id, ...d });
  });
}

function start() {
  if (boot) return boot;
  boot = (async () => {
    if (typeof WebAssembly !== "object" || typeof Worker !== "function") throw new ApiError(0, NEEDS);
    worker = new Worker(new URL("../worker.js", import.meta.url), { type: "module" });
    worker.onmessage = (e) => {
      const w = waits.get(e.data.id);
      if (!w) return;
      waits.delete(e.data.id);
      if (e.data.ok) w.ok(e.data);
      else w.bad(new ApiError(0, `The generator failed. ${e.data.message}`));
    };
    worker.onerror = (e) => {
      const msg = `The generator stopped. ${e.message || ""}`.trim();
      for (const w of waits.values()) w.bad(new ApiError(0, msg));
      waits.clear();
    };
    await call({ op: "hello" });
    if (in_place()) await restore((at, handle) => call({ op: "mount_handle", at, handle }));
  })();
  return boot;
}

export function mount_handle(at, handle) {
  return start().then(() => call({ op: "mount_handle", at, handle }));
}

export function mount_files(at, entries) {
  return start().then(() => call({ op: "mount_files", at, entries }));
}

export function init_token() {
  start().catch(() => {});
  return true;
}

async function raw(method, path, body) {
  if (path === "/api/open-folder" || path === "/api/open") {
    throw new ApiError(0, "The browser cannot open folders for you. Open the downloaded zip in your file manager.");
  }
  await start();
  const r = await call({ op: "req", method, path, body: body === undefined ? null : JSON.stringify(body) });
  if (method === "POST" && path === "/api/build") zip = null;
  if (r.zip) zip = r.zip;
  return r;
}

export async function api(method, path, body) {
  const r = await raw(method, path, body);
  let data = null;
  try {
    data = JSON.parse(new TextDecoder().decode(r.body));
  } catch {
    data = null;
  }
  if (r.status < 200 || r.status > 299) {
    throw new ApiError(r.status, (data && data.error) || `The generator answered ${r.status}.`);
  }
  return data;
}

export const get = (path) => api("GET", path);
export const post = (path, body) => api("POST", path, body);
export const q = (v) => encodeURIComponent(v);

export async function get_blob(path) {
  const r = await raw("GET", path);
  if (r.status < 200 || r.status > 299) throw new ApiError(r.status, `The generator answered ${r.status}.`);
  return new Blob([r.body], { type: r.ctype });
}
