import { $, h, fill } from "/js/dom.js";
import { get } from "/js/api.js";
import { state } from "/js/store.js";

const OFF = "ootrr.update.off";
const SKIP = "ootrr.update.skip";

function read(k) {
  try {
    return localStorage.getItem(k) || "";
  } catch {
    return "";
  }
}

function write(k, v) {
  try {
    localStorage.setItem(k, v);
  } catch {
    return;
  }
}

function banner(box, r) {
  const close = () => box.classList.add("hide");
  fill(
    box,
    h(
      "div",
      { class: "update-in" },
      h("span", { class: "update-text" }, h("b", null, `Version ${r.latest} is out`), ` (you have ${r.current}).`, r.note ? ` ${r.note}` : ""),
      h("a", { class: "btn", href: r.url, target: "_blank", rel: "noopener noreferrer" }, "Get it"),
      h("button", { type: "button", class: "btn quiet", onclick: () => {
        write(SKIP, r.latest);
        close();
      } }, "Not now"),
      h("button", { type: "button", class: "btn quiet", onclick: () => {
        write(OFF, "1");
        close();
      } }, "Stop checking"),
    ),
  );
  box.classList.remove("hide");
}

export async function start() {
  const box = $("#update");
  if (!box || !state.info || !state.info.update_check || read(OFF)) return;
  let r = null;
  try {
    r = await get("/api/update");
  } catch {
    return;
  }
  if (r && r.newer && r.url && read(SKIP) !== r.latest) banner(box, r);
}
