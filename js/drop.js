import { h } from "./dom.js";
import { can_drop, in_place, mount_entry } from "./api.js";

const BASE = "border:2px dashed var(--line);border-radius:8px;padding:12px 14px;margin-top:8px;font-size:13.5px;line-height:1.5;color:var(--muted)";
const OVER = "border:2px solid var(--accent);border-radius:8px;padding:12px 14px;margin-top:8px;font-size:13.5px;line-height:1.5;background:var(--accent-soft)";

export function drop_zone(set) {
  if (in_place() || !can_drop()) return null;
  const line = h("span", null, "Browse makes Firefox list every one of the game's files first (about 187,000 in the romfs folder), which takes many minutes and looks frozen. A dropped folder is read only as the build needs it. Nothing is uploaded.");
  const zone = h("div", { style: BASE, "aria-live": "polite" }, h("b", { style: "color:var(--fg)" }, "Drag the game's romfs folder here. "), line);
  const say = (t) => {
    line.textContent = t;
  };
  zone.addEventListener("dragover", (e) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
    zone.style.cssText = OVER;
  });
  zone.addEventListener("dragleave", () => {
    zone.style.cssText = BASE;
  });
  zone.addEventListener("drop", async (e) => {
    e.preventDefault();
    zone.style.cssText = BASE;
    const item = Array.from(e.dataTransfer.items || []).find((i) => i.kind === "file");
    const entry = item && item.webkitGetAsEntry();
    if (!entry) return say("Nothing usable was dropped. Drag the romfs folder from your file manager.");
    if (!entry.isDirectory) return say("That is a file. Drop the romfs folder here, or use Package… for a .nup or .nx.");
    const at = `/game/${entry.name}`;
    say(`Opening ${entry.name}…`);
    try {
      await mount_entry(at, entry);
    } catch (err) {
      return say(`The folder could not be opened: ${err.message}`);
    }
    say(`Using the dropped folder ${entry.name}. After a page reload, drop it again.`);
    set(at);
  });
  return zone;
}
