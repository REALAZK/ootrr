import { promises as fs } from "node:fs";
import { basename, join } from "node:path";

export class FileHandle {
  constructor(path) {
    this.kind = "file";
    this.path = path;
    this.name = basename(path);
  }

  async getFile() {
    const st = await fs.stat(this.path);
    const path = this.path;
    return {
      size: st.size,
      async arrayBuffer() {
        const b = await fs.readFile(path);
        return b.buffer.slice(b.byteOffset, b.byteOffset + b.length);
      },
    };
  }

  async createWritable() {
    const path = this.path;
    const parts = [];
    return {
      async write(b) {
        parts.push(Buffer.from(b));
      },
      async close() {
        await fs.writeFile(path, Buffer.concat(parts));
      },
    };
  }
}

export class DirHandle {
  constructor(path) {
    this.kind = "directory";
    this.path = path;
    this.name = basename(path);
  }

  async *entries() {
    for (const e of await fs.readdir(this.path, { withFileTypes: true })) {
      const p = join(this.path, e.name);
      yield [e.name, e.isDirectory() ? new DirHandle(p) : new FileHandle(p)];
    }
  }

  async getFileHandle(name, o = {}) {
    const p = join(this.path, name);
    if (o.create) await fs.writeFile(p, "", { flag: "a" });
    else await fs.access(p);
    return new FileHandle(p);
  }

  async getDirectoryHandle(name, o = {}) {
    const p = join(this.path, name);
    if (o.create) await fs.mkdir(p, { recursive: true });
    else await fs.access(p);
    return new DirHandle(p);
  }

  async removeEntry(name, o = {}) {
    try {
      await fs.rm(join(this.path, name), { recursive: !!o.recursive });
    } catch (e) {
      if (e.code === "ENOENT") {
        const err = new Error("not found");
        err.name = "NotFoundError";
        throw err;
      }
      throw e;
    }
  }
}
