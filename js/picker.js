import { in_place, mount_handle, mount_files, MODS } from "./api.js";
import { remember } from "./folders.js";

function pick_input(dir, accept) {
  return new Promise((ok) => {
    const input = document.createElement("input");
    input.type = "file";
    if (dir) {
      input.webkitdirectory = true;
      input.multiple = true;
    }
    if (accept) input.accept = accept;
    input.style.display = "none";
    const done = (files) => {
      input.remove();
      ok(files);
    };
    input.addEventListener("change", () => done(Array.from(input.files || [])));
    input.addEventListener("cancel", () => done([]));
    document.body.append(input);
    input.click();
  });
}

function kind_of(title) {
  if (/mod folder/i.test(title)) return "mod";
  if (/update/i.test(title)) return "update";
  return "game";
}

export async function pick_folder(_start, title) {
  const kind = kind_of(title);
  if (kind === "mod") return MODS;
  if (in_place()) {
    let handle;
    try {
      handle = await window.showDirectoryPicker({ id: `ootrr-${kind}`, mode: "read" });
    } catch {
      return null;
    }
    const at = `/${kind}/${handle.name}`;
    await mount_handle(at, handle);
    await remember(kind, at, handle);
    return at;
  }
  const files = await pick_input(true);
  if (!files.length) return null;
  const top = (files[0].webkitRelativePath || files[0].name).split("/")[0];
  const entries = files.map((f) => [(f.webkitRelativePath || f.name).split("/").slice(1).join("/"), f]);
  const at = `/${kind}/${top}`;
  await mount_files(at, entries);
  return at;
}

export async function pick_package(_start, title) {
  const kind = kind_of(title);
  const [f] = await pick_input(false, ".nup,.nx");
  if (!f) return null;
  const at = `/${kind}-package`;
  await mount_files(at, [[f.name, f]]);
  return `${at}/${f.name}`;
}
