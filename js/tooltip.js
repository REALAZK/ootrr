import { $, h, fill } from "./dom.js";

let owner = null;

function box() {
  return $("#tip");
}

function place(btn) {
  const t = box();
  const r = btn.getBoundingClientRect();
  const x = Math.min(Math.max(8, r.left - 12), window.innerWidth - t.offsetWidth - 8);
  let y = r.bottom + 8;
  if (y + t.offsetHeight > window.innerHeight - 8) y = Math.max(8, r.top - t.offsetHeight - 8);
  t.style.left = `${x}px`;
  t.style.top = `${y}px`;
}

export function hide() {
  const t = box();
  if (t) t.hidden = true;
  owner = null;
}

function show(btn, content) {
  const t = box();
  if (!t) return;
  fill(t, content());
  t.hidden = false;
  owner = btn;
  place(btn);
}

export function tip_button(label, content) {
  const b = h("button", { type: "button", class: "qb", "aria-label": `About ${label}` }, "?");
  b.addEventListener("mouseenter", () => show(b, content));
  b.addEventListener("mouseleave", hide);
  b.addEventListener("focus", () => show(b, content));
  b.addEventListener("blur", hide);
  b.addEventListener("click", () => (owner === b ? hide() : show(b, content)));
  return b;
}

export function start() {
  window.addEventListener("scroll", hide, { passive: true, capture: true });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") hide();
  });
}
