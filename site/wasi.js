const E = {
  OK: 0,
  BADF: 8,
  EXIST: 20,
  INVAL: 28,
  ISDIR: 31,
  NOENT: 44,
  NOTDIR: 54,
  NOTEMPTY: 55,
  ROFS: 69,
};

const KIND = { CHAR: 2, DIR: 3, FILE: 4 };
const O = { CREAT: 1, DIRECTORY: 2, EXCL: 4, TRUNC: 8 };
const APPEND = 1;
const WRITE = 0x40n;
const ALL = 0xffffffffffffffffn;

class Fail extends Error {
  constructor(code) {
    super(`wasi errno ${code}`);
    this.code = code;
  }
}

export class Exit extends Error {
  constructor(code) {
    super(`exit ${code}`);
    this.code = code;
  }
}

let next_ino = 1n;

export class Dir {
  constructor(parent, ro) {
    this.kind = KIND.DIR;
    this.parent = parent || this;
    this.children = new Map();
    this.ro = !!ro;
    this.ino = next_ino++;
  }

  load() {}
}

export class HDir extends Dir {
  constructor(parent, handle) {
    super(parent, true);
    this.handle = handle;
    this.loaded = null;
  }

  load() {
    if (this.loaded === true) return undefined;
    if (!this.loaded) {
      this.loaded = (async () => {
        for await (const [name, h] of this.handle.entries()) {
          this.children.set(name, h.kind === "directory" ? new HDir(this, h) : new HFile(this, h));
        }
        this.loaded = true;
      })();
    }
    return this.loaded;
  }
}

export class Mem {
  constructor(parent, bytes) {
    this.kind = KIND.FILE;
    this.parent = parent;
    this.buf = bytes || new Uint8Array(0);
    this.size = this.buf.length;
    this.ro = false;
    this.ino = next_ino++;
  }

  load() {}

  read() {
    return this.bytes();
  }

  bytes() {
    return this.buf.subarray(0, this.size);
  }

  write_at(pos, b) {
    const end = pos + b.length;
    if (end > this.buf.length) {
      const grown = new Uint8Array(Math.max(end, this.buf.length * 2, 4096));
      grown.set(this.buf.subarray(0, this.size));
      this.buf = grown;
    }
    this.buf.set(b, pos);
    if (end > this.size) this.size = end;
  }

  truncate() {
    this.buf = new Uint8Array(0);
    this.size = 0;
  }
}

export class HFile {
  constructor(parent, handle) {
    this.kind = KIND.FILE;
    this.parent = parent;
    this.handle = handle;
    this.ro = true;
    this.ino = next_ino++;
    this.file = null;
    this.size = 0;
  }

  load() {
    if (this.file) return undefined;
    return this.handle.getFile().then((f) => {
      this.file = f;
      this.size = f.size;
    });
  }

  read() {
    return this.load_then(() => this.file.arrayBuffer().then((b) => new Uint8Array(b)));
  }

  load_then(f) {
    const p = this.load();
    return p ? p.then(f) : f();
  }
}

export class SFile {
  constructor(parent, src, reader) {
    this.kind = KIND.FILE;
    this.parent = parent;
    this.src = src;
    this.reader = reader;
    this.size = src.size;
    this.ro = true;
    this.ino = next_ino++;
  }

  load() {}

  read() {
    return this.reader(this.src);
  }
}

function split(p) {
  return p.split("/").filter((s) => s.length > 0 && s !== ".");
}

export function dir_at(root, at, make) {
  let d = root;
  for (const s of split(at)) {
    let c = d.children.get(s);
    if (!c) {
      c = make ? make(d) : new Dir(d, false);
      d.children.set(s, c);
    }
    d = c;
  }
  return d;
}

export function mount_handle(root, at, handle) {
  const parts = split(at);
  const name = parts.pop();
  const d = dir_at(root, parts.join("/"));
  const m = new HDir(d, handle);
  d.children.set(name, m);
  return m;
}

export function mount_files(root, at, entries, reader) {
  const parts = split(at);
  const name = parts.pop();
  const d = dir_at(root, parts.join("/"));
  const base = new Dir(d, true);
  d.children.set(name, base);
  const ro = (p) => new Dir(p, true);
  for (const [p, src] of entries) {
    const segs = split(p);
    const file = segs.pop();
    const at_dir = segs.length ? dir_at(base, segs.join("/"), ro) : base;
    at_dir.children.set(file, new SFile(at_dir, src, reader));
  }
  return base;
}

export function files_under(d, prefix, out) {
  for (const [k, v] of d.children) {
    const p = prefix ? `${prefix}/${k}` : k;
    if (v.kind !== KIND.DIR) out.push([p, v.bytes()]);
    else if (v.children.size) files_under(v, p, out);
    else out.push([`${p}/`, null]);
  }
  return out;
}

function run_sync(g) {
  let r = g.next();
  while (!r.done) {
    if (r.value && typeof r.value.then === "function") throw new Error("a file needed waiting in the synchronous mode");
    r = g.next(r.value);
  }
  return r.value;
}

async function run_async(g) {
  let r = g.next();
  while (!r.done) r = g.next(await r.value);
  return r.value;
}

export class Wasi {
  constructor(o) {
    this.args = o.args;
    this.env = o.env || [];
    this.root = o.root;
    this.async = !!o.async;
    this.stdin = o.stdin;
    this.out = o.stdout || (() => {});
    this.err = o.stderr || (() => {});
    this.fds = new Map([
      [0, { std: 0 }],
      [1, { std: 1 }],
      [2, { std: 2 }],
      [3, { node: this.root, pos: 0, pre: "/" }],
    ]);
    this.next_fd = 4;
    this.mem = null;
    this.enc = new TextEncoder();
    this.dec = new TextDecoder();
  }

  start(inst) {
    this.mem = inst.exports.memory;
    const done = (e) => {
      if (e instanceof Exit) return e.code;
      throw e;
    };
    if (this.async) return WebAssembly.promising(inst.exports._start)().then(() => 0, done);
    try {
      inst.exports._start();
      return 0;
    } catch (e) {
      return done(e);
    }
  }

  view() {
    return new DataView(this.mem.buffer);
  }

  u8() {
    return new Uint8Array(this.mem.buffer);
  }

  str(ptr, len) {
    return this.dec.decode(this.u8().slice(ptr, ptr + len));
  }

  fd(n) {
    const f = this.fds.get(n);
    if (!f) throw new Fail(E.BADF);
    return f;
  }

  dir_of(n) {
    const f = this.fd(n);
    if (!f.node || f.node.kind !== KIND.DIR) throw new Fail(E.NOTDIR);
    return f.node;
  }

  *resolve(dir, p) {
    let n = dir;
    for (const s of split(p)) {
      if (n.kind !== KIND.DIR) throw new Fail(E.NOTDIR);
      yield n.load();
      n = s === ".." ? n.parent : n.children.get(s);
      if (!n) throw new Fail(E.NOENT);
    }
    return n;
  }

  *parent_of(dir, p) {
    const parts = split(p);
    const name = parts.pop();
    if (!name || name === "..") throw new Fail(E.INVAL);
    const d = yield* this.resolve(dir, parts.join("/"));
    if (d.kind !== KIND.DIR) throw new Fail(E.NOTDIR);
    yield d.load();
    return [d, name];
  }

  stat_to(ptr, n) {
    const v = this.view();
    v.setBigUint64(ptr, 0n, true);
    v.setBigUint64(ptr + 8, n.ino, true);
    v.setUint8(ptr + 16, n.kind);
    v.setBigUint64(ptr + 24, 1n, true);
    v.setBigUint64(ptr + 32, BigInt(n.kind === KIND.DIR ? 0 : n.size), true);
    v.setBigUint64(ptr + 40, 0n, true);
    v.setBigUint64(ptr + 48, 0n, true);
    v.setBigUint64(ptr + 56, 0n, true);
  }

  iovs(ptr, len) {
    const v = this.view();
    const out = [];
    for (let i = 0; i < len; i++) out.push([v.getUint32(ptr + i * 8, true), v.getUint32(ptr + i * 8 + 4, true)]);
    return out;
  }

  fill(iov, iov_len, data, from) {
    let at = from;
    for (const [ptr, len] of this.iovs(iov, iov_len)) {
      const k = Math.min(len, data.length - at);
      if (k <= 0) break;
      this.u8().set(data.subarray(at, at + k), ptr);
      at += k;
    }
    return at - from;
  }

  imports() {
    const now = (f) => (...a) => {
      try {
        return f(...a);
      } catch (e) {
        if (e instanceof Fail) return e.code;
        throw e;
      }
    };
    const io = (gen) => {
      if (this.async) {
        return new WebAssembly.Suspending(async (...a) => {
          try {
            return await run_async(gen(...a));
          } catch (e) {
            if (e instanceof Fail) return e.code;
            throw e;
          }
        });
      }
      return now((...a) => run_sync(gen(...a)));
    };
    const list = (items, ptr_ptr, buf_ptr) => {
      const v = this.view();
      const m = this.u8();
      let at = buf_ptr;
      items.forEach((s, i) => {
        const b = this.enc.encode(`${s}\0`);
        v.setUint32(ptr_ptr + i * 4, at, true);
        m.set(b, at);
        at += b.length;
      });
      return E.OK;
    };
    const sizes = (items, n_ptr, len_ptr) => {
      const v = this.view();
      v.setUint32(n_ptr, items.length, true);
      v.setUint32(len_ptr, items.reduce((t, s) => t + this.enc.encode(s).length + 1, 0), true);
      return E.OK;
    };
    const self = this;
    return {
      wasi_snapshot_preview1: {
        args_sizes_get: now((n, l) => sizes(this.args, n, l)),
        args_get: now((p, b) => list(this.args, p, b)),
        environ_sizes_get: now((n, l) => sizes(this.env, n, l)),
        environ_get: now((p, b) => list(this.env, p, b)),
        random_get: now((ptr, len) => {
          const m = this.u8();
          for (let i = 0; i < len; i += 65536) crypto.getRandomValues(m.subarray(ptr + i, ptr + Math.min(len, i + 65536)));
          return E.OK;
        }),
        clock_time_get: now((id, _prec, ptr) => {
          const ms = id === 0 ? Date.now() : performance.now();
          this.view().setBigUint64(ptr, BigInt(Math.round(ms * 1e6)), true);
          return E.OK;
        }),
        sched_yield: () => E.OK,
        poll_oneoff: now((sin, sout, n, nout) => {
          const v = this.view();
          for (let i = 0; i < n; i++) {
            const o = sout + i * 32;
            v.setBigUint64(o, v.getBigUint64(sin + i * 48, true), true);
            v.setUint16(o + 8, 0, true);
            v.setUint8(o + 10, v.getUint8(sin + i * 48 + 8));
          }
          v.setUint32(nout, n, true);
          return E.OK;
        }),
        proc_exit: (code) => {
          throw new Exit(code);
        },
        fd_close: now((n) => {
          this.fd(n);
          if (n > 3) this.fds.delete(n);
          return E.OK;
        }),
        fd_fdstat_get: now((n, ptr) => {
          const f = this.fd(n);
          const v = this.view();
          v.setUint8(ptr, f.std !== undefined ? KIND.CHAR : f.node.kind);
          v.setUint16(ptr + 2, f.append ? APPEND : 0, true);
          v.setBigUint64(ptr + 8, ALL, true);
          v.setBigUint64(ptr + 16, ALL, true);
          return E.OK;
        }),
        fd_filestat_get: io(function* (n, ptr) {
          const f = self.fd(n);
          if (f.std !== undefined) {
            self.stat_to(ptr, { ino: 0n, kind: KIND.CHAR, size: 0 });
            return E.OK;
          }
          yield f.node.load();
          self.stat_to(ptr, f.node);
          return E.OK;
        }),
        fd_prestat_get: now((n, ptr) => {
          const f = this.fd(n);
          if (!f.pre) return E.BADF;
          const v = this.view();
          v.setUint8(ptr, 0);
          v.setUint32(ptr + 4, this.enc.encode(f.pre).length, true);
          return E.OK;
        }),
        fd_prestat_dir_name: now((n, ptr, len) => {
          const f = this.fd(n);
          if (!f.pre) return E.BADF;
          this.u8().set(this.enc.encode(f.pre).subarray(0, len), ptr);
          return E.OK;
        }),
        fd_read: io(function* (n, iov, iov_len, nread) {
          const f = self.fd(n);
          let k = 0;
          if (f.std === 0) {
            const b = self.stdin(self.iovs(iov, iov_len).reduce((t, [, l]) => t + l, 0));
            k = self.fill(iov, iov_len, b, 0);
          } else if (f.std === undefined) {
            if (f.node.kind === KIND.DIR) return E.ISDIR;
            if (!f.data && !(f.node instanceof Mem)) f.data = yield f.node.read();
            k = self.fill(iov, iov_len, f.data || f.node.bytes(), f.pos);
            f.pos += k;
          }
          self.view().setUint32(nread, k, true);
          return E.OK;
        }),
        fd_write: now((n, iov, iov_len, nwritten) => {
          const f = this.fd(n);
          let total = 0;
          for (const [ptr, len] of this.iovs(iov, iov_len)) {
            const b = this.u8().slice(ptr, ptr + len);
            if (f.std === 1) this.out(b);
            else if (f.std === 2) this.err(b);
            else if (f.std === 0 || !(f.node instanceof Mem)) return E.BADF;
            else {
              if (f.append) f.pos = f.node.size;
              f.node.write_at(f.pos, b);
              f.pos += b.length;
            }
            total += len;
          }
          this.view().setUint32(nwritten, total, true);
          return E.OK;
        }),
        fd_readdir: io(function* (n, buf, buf_len, cookie, used) {
          const d = self.dir_of(n);
          yield d.load();
          const names = [...d.children.keys()];
          const m = self.u8();
          let at = 0;
          for (let i = Number(cookie); i < names.length && at < buf_len; i++) {
            const name = self.enc.encode(names[i]);
            const c = d.children.get(names[i]);
            const ent = new Uint8Array(24 + name.length);
            const ev = new DataView(ent.buffer);
            ev.setBigUint64(0, BigInt(i + 1), true);
            ev.setBigUint64(8, c.ino, true);
            ev.setUint32(16, name.length, true);
            ev.setUint8(20, c.kind);
            ent.set(name, 24);
            const k = Math.min(ent.length, buf_len - at);
            m.set(ent.subarray(0, k), buf + at);
            at += k;
          }
          self.view().setUint32(used, at, true);
          return E.OK;
        }),
        path_create_directory: io(function* (dfd, p, l) {
          const [d, name] = yield* self.parent_of(self.dir_of(dfd), self.str(p, l));
          if (d.ro) return E.ROFS;
          if (d.children.has(name)) return E.EXIST;
          d.children.set(name, new Dir(d, false));
          return E.OK;
        }),
        path_filestat_get: io(function* (dfd, _flags, p, l, ptr) {
          const node = yield* self.resolve(self.dir_of(dfd), self.str(p, l));
          yield node.load();
          self.stat_to(ptr, node);
          return E.OK;
        }),
        path_open: io(function* (dfd, _dirflags, p, l, oflags, rights, _inh, fdflags, out) {
          const base = self.dir_of(dfd);
          const path = self.str(p, l);
          const writes = (rights & WRITE) !== 0n || (oflags & O.TRUNC) !== 0;
          let node;
          try {
            node = yield* self.resolve(base, path);
            if (oflags & O.EXCL && oflags & O.CREAT) return E.EXIST;
          } catch (e) {
            if (!(e instanceof Fail) || e.code !== E.NOENT || !(oflags & O.CREAT)) throw e;
            const [d, name] = yield* self.parent_of(base, path);
            if (d.ro) return E.ROFS;
            node = new Mem(d);
            d.children.set(name, node);
          }
          if (oflags & O.DIRECTORY && node.kind !== KIND.DIR) return E.NOTDIR;
          if (node.kind === KIND.DIR && writes) return E.ISDIR;
          if (node.ro && writes) return E.ROFS;
          if (oflags & O.TRUNC && node instanceof Mem) node.truncate();
          const n = self.next_fd++;
          self.fds.set(n, { node, pos: 0, append: !!(fdflags & APPEND) });
          self.view().setUint32(out, n, true);
          return E.OK;
        }),
        path_readlink: now(() => E.INVAL),
        path_remove_directory: io(function* (dfd, p, l) {
          const [d, name] = yield* self.parent_of(self.dir_of(dfd), self.str(p, l));
          const c = d.children.get(name);
          if (!c) return E.NOENT;
          if (c.kind !== KIND.DIR) return E.NOTDIR;
          if (d.ro) return E.ROFS;
          yield c.load();
          if (c.children.size) return E.NOTEMPTY;
          d.children.delete(name);
          return E.OK;
        }),
        path_unlink_file: io(function* (dfd, p, l) {
          const [d, name] = yield* self.parent_of(self.dir_of(dfd), self.str(p, l));
          const c = d.children.get(name);
          if (!c) return E.NOENT;
          if (c.kind === KIND.DIR) return E.ISDIR;
          if (d.ro) return E.ROFS;
          d.children.delete(name);
          return E.OK;
        }),
        path_rename: io(function* (ofd, op, ol, nfd, np, nl) {
          const [od, oname] = yield* self.parent_of(self.dir_of(ofd), self.str(op, ol));
          const [nd, nname] = yield* self.parent_of(self.dir_of(nfd), self.str(np, nl));
          const c = od.children.get(oname);
          if (!c) return E.NOENT;
          if (od.ro || nd.ro) return E.ROFS;
          const t = nd.children.get(nname);
          if (t && t.kind === KIND.DIR && t.children.size) return E.NOTEMPTY;
          if (t && t.kind !== c.kind) return t.kind === KIND.DIR ? E.ISDIR : E.NOTDIR;
          od.children.delete(oname);
          c.parent = nd;
          nd.children.set(nname, c);
          return E.OK;
        }),
      },
    };
  }
}
