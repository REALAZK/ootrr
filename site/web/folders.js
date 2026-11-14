const DB = "ootrr-web";
const STORE = "folders";

function open_db() {
  return new Promise((ok, bad) => {
    const r = indexedDB.open(DB, 1);
    r.onupgradeneeded = () => r.result.createObjectStore(STORE);
    r.onsuccess = () => ok(r.result);
    r.onerror = () => bad(r.error);
  });
}

async function with_store(mode, f) {
  const db = await open_db();
  try {
    return await new Promise((ok, bad) => {
      const t = db.transaction(STORE, mode);
      const r = f(t.objectStore(STORE));
      t.oncomplete = () => ok(r && r.result);
      t.onerror = () => bad(t.error);
    });
  } finally {
    db.close();
  }
}

export async function remember(kind, at, handle) {
  try {
    await with_store("readwrite", (s) => s.put({ at, handle }, kind));
  } catch {
    return;
  }
}

export async function restore(mount) {
  let all = [];
  try {
    all = (await with_store("readonly", (s) => s.getAll())) || [];
  } catch {
    return;
  }
  for (const f of all) {
    try {
      if (!f.rw && (await f.handle.queryPermission({ mode: "read" })) === "granted") await mount(f.at, f.handle);
    } catch {
      continue;
    }
  }
}
