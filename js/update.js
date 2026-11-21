import { $, h, fill } from "./dom.js";
import { get } from "./api.js";
import { state } from "./store.js";

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
      h("span", { class: "update-text" }, h("b", null, `Version ${r.latest} is out.`), ` You have ${r.current}.`),
      h("a", { class: "btn", href: r.url, target: "_blank", rel: "noopener noreferrer" }, "Get it"),
      h("button", { type: "button", class: "btn quiet", onclick: () => {
        write(SKIP, r.latest);
        close();
      } }, "Not now"),
    ),
  );
  box.classList.remove("hide");
}

export async function start() {
  const box = $("#update");
  if (!box || !state.info || !state.info.update_check) return;
  let r = null;
  try {
    r = await get("/api/update");
  } catch {
    return;
  }
  if (r && r.newer && r.url && read(SKIP) !== r.latest) banner(box, r);
}
