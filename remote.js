class RemoteFile {
  constructor(call, root, path, name) {
    this.kind = "file";
    this.call = call;
    this.root = root;
    this.path = path;
    this.name = name;
  }

  getFile() {
    return this.call({ fs: "file", root: this.root, path: this.path });
  }
}

class RemoteDir {
  constructor(call, root, path, name) {
    this.kind = "directory";
    this.call = call;
    this.root = root;
    this.path = path;
    this.name = name;
  }

  async *entries() {
    const list = await this.call({ fs: "list", root: this.root, path: this.path });
    for (const [name, dir] of list) {
      const p = this.path ? `${this.path}/${name}` : name;
      yield [name, dir ? new RemoteDir(this.call, this.root, p, name) : new RemoteFile(this.call, this.root, p, name)];
    }
  }
}

export function remote_root(call, root, name) {
  return new RemoteDir(call, root, "", name);
}

export function remote_caller(post) {
  let seq = 0;
  const waits = new Map();
  const call = (req) =>
    new Promise((ok, bad) => {
      const n = ++seq;
      waits.set(n, { ok, bad });
      post({ fs_ask: n, ...req });
    });
  const answer = (d) => {
    const w = waits.get(d.fs_reply);
    if (!w) return;
    waits.delete(d.fs_reply);
    if (d.ok) w.ok(d.data);
    else w.bad(new Error(d.message));
  };
  return { call, answer };
}

const cb = (fn) => new Promise((ok, bad) => fn(ok, bad));

function dir_at(root, path) {
  return path ? cb((ok, bad) => root.getDirectory(path, {}, ok, bad)) : Promise.resolve(root);
}

async function list(root, path) {
  const d = await dir_at(root, path);
  const reader = d.createReader();
  const out = [];
  for (;;) {
    const batch = await cb((ok, bad) => reader.readEntries(ok, bad));
    if (!batch.length) return out;
    for (const e of batch) out.push([e.name, e.isDirectory]);
  }
}

async function file(root, path) {
  const fe = await cb((ok, bad) => root.getFile(path, {}, ok, bad));
  return cb((ok, bad) => fe.file(ok, bad));
}

export async function serve_entry(root, d) {
  if (!root) throw new Error("the dropped folder is no longer open, drop it again");
  if (d.fs === "list") return list(root, d.path);
  if (d.fs === "file") return file(root, d.path);
  throw new Error(`unknown folder request ${d.fs}`);
}
