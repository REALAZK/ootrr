import { $, h, fill, toast } from "./dom.js";
import { t_mark } from "./tracker_api.js";

const pending = [];
let timer = 0;

function hide() {
  pending.length = 0;
  const box = $("#news");
  if (box) box.replaceChildren();
}

async function undo() {
  const ids = pending.map((l) => l.id);
  hide();
  let n = 0;
  for (const id of ids) {
    try {
      await t_mark(id, false);
      n += 1;
    } catch (e) {
      toast(e.message);
    }
  }
  if (n) toast(n === 1 ? "Undone. The check is open again." : `Undone. ${n} checks are open again.`);
}

function paint() {
  const box = $("#news");
  if (!box) return;
  const shown = pending.slice(0, 6);
  const more = pending.length - shown.length;
  fill(
    box,
    h(
      "div",
      { class: "nb" },
      h("h3", null, pending.length === 1 ? "New from your save" : `${pending.length} new from your save`),
      h("ul", null, shown.map((l) => h("li", null, l.label, l.area ? h("span", { class: "muted" }, ` (${l.area})`) : null))),
      more > 0 ? h("p", { class: "muted small" }, `and ${more} more`) : null,
      h(
        "div",
        { class: "row" },
        h("button", { type: "button", class: "btn small", onclick: undo }, "Undo"),
        h("button", { type: "button", class: "btn small", onclick: hide }, "Close"),
      ),
    ),
  );
}

export function announce(locs) {
  if (!locs.length) return;
  for (const l of locs) {
    if (!pending.some((p) => p.id === l.id)) pending.push(l);
  }
  paint();
  clearTimeout(timer);
  timer = setTimeout(hide, 15000);
}
