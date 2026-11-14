import { Wasi, Dir, dir_at, mount_handle, mount_files, files_under } from "./wasi.js";

const HOST = "127.0.0.1:1";
const OUT = "zip/mods";
const enc = new TextEncoder();
const dec = new TextDecoder();

export const MODS = `/${OUT}`;

function token() {
  const a = "abcdefghjkmnpqrstuvwxyz23456789";
  return Array.from(crypto.getRandomValues(new Uint8Array(32)), (x) => a[x % a.length]).join("");
}

function find(b, seq, from) {
  outer: for (let i = from; i + seq.length <= b.length; i++) {
    for (let j = 0; j < seq.length; j++) if (b[i + j] !== seq[j]) continue outer;
    return i;
  }
  return -1;
}

const CRLF = enc.encode("\r\n");
const END = enc.encode("\r\n\r\n");

function frame(tok, method, path, body) {
  const b = body === null || body === undefined ? null : enc.encode(body);
  let head = `${method} ${path} HTTP/1.1\r\nHost: ${HOST}\r\nX-OOTRR-Token: ${tok}\r\nSec-Fetch-Site: same-origin\r\n`;
  if (method === "POST") head += `Origin: http://${HOST}\r\nContent-Type: application/json\r\nContent-Length: ${b ? b.length : 0}\r\n`;
  const h = enc.encode(`${head}\r\n`);
  if (!b) return h;
  const out = new Uint8Array(h.length + b.length);
  out.set(h);
  out.set(b, h.length);
  return out;
}

function responses(b) {
  let at = find(b, CRLF, 0) + 2;
  const out = [];
  while (at > 1 && at < b.length) {
    const end = find(b, END, at);
    if (end < 0) break;
    const head = dec.decode(b.subarray(at, end)).split("\r\n");
    const field = (k) => head.find((x) => x.toLowerCase().startsWith(`${k}:`))?.slice(k.length + 1).trim() || "";
    const len = Number(field("content-length"));
    out.push({ status: Number(head[0].split(" ")[1]), ctype: field("content-type"), body: b.slice(end + 4, end + 4 + len) });
    at = end + 4 + len;
  }
  return out;
}

function joined(parts) {
  const out = new Uint8Array(parts.reduce((t, p) => t + p.length, 0));
  let at = 0;
  for (const p of parts) {
    out.set(p, at);
    at += p.length;
  }
  return out;
}

export class Host {
  constructor(module, o = {}) {
    this.module = module;
    this.async = !!o.async;
    this.reader = o.reader;
    this.log = o.log || (() => {});
    this.root = new Dir(null, false);
    dir_at(this.root, OUT);
    this.jobs = new Map();
    this.built = null;
    this.chain = Promise.resolve();
  }

  run(input) {
    const tok = token();
    const stdin = joined(input.map((x) => frame(tok, ...x)));
    let at = 0;
    const out = [];
    const err = new TextDecoder();
    const wasi = new Wasi({
      args: ["ootrr", "serve", "--stdio"],
      env: [`OOTRR_STDIO_TOKEN=${tok}`],
      root: this.root,
      async: this.async,
      stdin: (max) => {
        const b = stdin.subarray(at, Math.min(stdin.length, at + max));
        at += b.length;
        return b;
      },
      stdout: (b) => out.push(b),
      stderr: (b) => this.log(err.decode(b, { stream: true })),
    });
    const inst = new WebAssembly.Instance(this.module, wasi.imports());
    const finish = (code) => {
      const rs = responses(joined(out));
      if (rs.length < input.length) throw new Error(`the generator stopped with exit code ${code} before it answered`);
      return rs;
    };
    const r = wasi.start(inst);
    return this.async ? r.then(finish) : finish(r);
  }

  async once(method, path, body) {
    const job = /^\/api\/jobs\/(\d+)$/.exec(path);
    if (job && method === "GET" && this.jobs.has(job[1])) return { ...this.jobs.get(job[1]), built: this.built };
    const build = method === "POST" && path === "/api/build";
    if (build) {
      dir_at(this.root, "zip").children.delete("mods");
      dir_at(this.root, OUT);
      this.built = null;
    }
    const input = [[method, path, body]];
    if (build) input.push(["GET", "/api/jobs/1", null]);
    const [r, j] = await this.run(input);
    if (build && j && r.status === 202) {
      this.jobs.clear();
      this.jobs.set("1", j);
      const state = JSON.parse(dec.decode(j.body)).state;
      if (state === "done") this.built = files_under(dir_at(this.root, OUT), "", []);
    }
    return r;
  }

  request(method, path, body) {
    const p = this.chain.then(() => this.once(method, path, body));
    this.chain = p.catch(() => {});
    return p;
  }

  mount_handle(at, handle) {
    return this.chain.then(() => mount_handle(this.root, at, handle));
  }

  mount_files(at, entries) {
    return this.chain.then(() => mount_files(this.root, at, entries, this.reader));
  }
}
