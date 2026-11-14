import { in_place, mount_handle, mount_files } from "./api.js";
import { remember } from "./folders.js";

function pick_files() {
  return new Promise((ok) => {
    const input = document.createElement("input");
    input.type = "file";
    input.webkitdirectory = true;
    input.multiple = true;
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

export async function pick_folder(_current, title, on_wait) {
  if (!/game folder/i.test(title)) return null;
  if (in_place()) {
    let handle;
    try {
      handle = await window.showDirectoryPicker({ id: "ootrr-game", mode: "read" });
    } catch {
      return null;
    }
    const at = `/game/${handle.name}`;
    await mount_handle(at, handle);
    await remember("game", at, handle);
    return at;
  }
  const files = await pick_files();
  if (!files.length) return null;
  if (on_wait) on_wait(files.length);
  const top = (files[0].webkitRelativePath || files[0].name).split("/")[0];
  const entries = files.map((f) => [(f.webkitRelativePath || f.name).split("/").slice(1).join("/"), f]);
  const at = `/game/${top}`;
  await mount_files(at, entries);
  return at;
}
