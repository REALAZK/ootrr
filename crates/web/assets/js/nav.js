import { $, h, fill } from "./dom.js";
import { state } from "./store.js";

export const STEPS = [
  { id: "setup", label: "Setup" },
  { id: "settings", label: "Settings" },
  { id: "generate", label: "Generate" },
  { id: "result", label: "Result" },
];

const screens = new Map();
let current = "";

export function register(id, render) {
  screens.set(id, render);
}

export function here() {
  const m = /^#\/([a-z]+)/.exec(location.hash);
  return m && screens.has(m[1]) ? m[1] : "setup";
}

export function go(id) {
  if (here() === id && current === id) return;
  location.hash = `#/${id}`;
}

function done(id) {
  if (id === "setup") return Boolean(state.game_check && state.game_check.ok && state.mod_check && state.mod_check.ok);
  if (id === "generate") return Boolean(state.result);
  return false;
}

export function bar() {
  const box = $("#steps");
  if (!box) return;
  const kids = STEPS.map((s, i) =>
    h(
      "button",
      {
        type: "button",
        class: `step${done(s.id) ? " done" : ""}`,
        "aria-current": s.id === current ? "step" : null,
        onclick: () => go(s.id),
      },
      h("span", { class: "n", "aria-hidden": "true" }, done(s.id) ? "✓" : String(i + 1)),
      s.label,
    ),
  );
  fill(box, kids);
}

export function show() {
  const id = here();
  current = id;
  const main = $("#main");
  try {
    fill(main, screens.get(id)());
  } catch (e) {
    fill(
      main,
      h("p", { class: "notice error", role: "alert" }, `This screen could not be shown. ${e.message}`),
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
  bar();
  window.scrollTo(0, 0);
  main.focus({ preventScroll: true });
}

export function start() {
  window.addEventListener("hashchange", show);
  show();
}
