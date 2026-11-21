import { $, fill, h } from "./dom.js";
import { state } from "./store.js";
import { hide as hide_tip } from "./tooltip.js";

const OLD = { setup: "config", settings: "world", generate: "seed", result: "seed" };

const screens = new Map();
let first = "config";
let current = "";
let after = () => {};

export function register(id, render) {
  screens.set(id, render);
}

export function set_first(id) {
  first = id;
}

export function on_show(fn) {
  after = fn;
}

export function here() {
  const m = /^#\/([a-z_]+)/.exec(location.hash);
  if (!m) return first;
  const id = OLD[m[1]] && !screens.has(m[1]) ? OLD[m[1]] : m[1];
  return screens.has(id) ? id : first;
}

export function current_page() {
  return current;
}

export function go(id) {
  if (here() === id && current === id) return show();
  location.hash = `#/${id}`;
}

export function show() {
  const id = here();
  const again = id === current;
  current = id;
  hide_tip();
  const main = $("#main");
  try {
    fill(main, screens.get(id)());
  } catch (e) {
    fill(
      main,
      h("p", { class: "notice error", role: "alert" }, `This page could not be shown. ${e.message}`),
      h("button", { type: "button", class: "btn", onclick: () => {
        state.result = null;
        try {
          sessionStorage.removeItem("ootrr.result");
        } catch {
          state.result = null;
        }
        show();
      } }, "Clear the saved seed and try again"),
    );
  }
  after(id);
  if (!again) window.scrollTo(0, 0);
}

export function start() {
  window.addEventListener("hashchange", show);
  show();
}
