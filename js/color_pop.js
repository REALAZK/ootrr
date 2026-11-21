import { h } from "./dom.js";

const KEY = "ootrr.recent_colors";
const HEX = /^#[0-9a-f]{6}$/;
let pop = null;

function tinted(el, c) {
  el.style.background = c;
  return el;
}

function to_hex(hu, s, v) {
  const f = (n) => {
    const k = (n + hu / 60) % 6;
    return v - v * s * Math.max(0, Math.min(k, 4 - k, 1));
  };
  return `#${[f(5), f(3), f(1)].map((x) => Math.round(x * 255).toString(16).padStart(2, "0")).join("")}`;
}

function of_hex(hex) {
  const n = parseInt(hex.slice(1), 16);
  const [r, g, b] = [(n >> 16) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255];
  const hi = Math.max(r, g, b);
  const d = hi - Math.min(r, g, b);
  let hu = 0;
  if (d > 0) {
    if (hi === r) hu = 60 * (((g - b) / d + 6) % 6);
    else if (hi === g) hu = 60 * ((b - r) / d + 2);
    else hu = 60 * ((r - g) / d + 4);
  }
  return [hu, hi > 0 ? d / hi : 0, hi];
}

function recent() {
  try {
    const v = JSON.parse(localStorage.getItem(KEY) || "[]");
    return Array.isArray(v) ? v.filter((x) => HEX.test(x)).slice(0, 8) : [];
  } catch {
    return [];
  }
}

function remember(hex) {
  const v = [hex, ...recent().filter((x) => x !== hex)].slice(0, 8);
  try {
    localStorage.setItem(KEY, JSON.stringify(v));
  } catch {
    return;
  }
}

export function close_picker() {
  if (pop) pop.close();
}

export function open_picker(anchor, { title, value, onchange, onreset }) {
  close_picker();
  let [hu, s, v] = of_hex(HEX.test(value) ? value : "#808080");
  const start = value;
  let now = value;

  const sv_th = h("span", { class: "cpop-th" });
  const sv = h("div", { class: "cpop-sv", tabindex: "0", role: "slider", "aria-label": "Saturation and brightness" }, sv_th);
  const hue_th = h("span", { class: "cpop-th" });
  const hue = h("div", { class: "cpop-hue", tabindex: "0", role: "slider", "aria-label": "Hue", "aria-valuemin": "0", "aria-valuemax": "360" }, hue_th);
  const prev = h("span", { class: "cpop-prev", "aria-hidden": "true" });
  const hex = h("input", { type: "text", class: "cpop-hex", maxlength: "7", spellcheck: "false", autocomplete: "off", "aria-label": "Hex colour" });
  const used = recent();

  const paint = (typed) => {
    now = to_hex(hu, s, v);
    sv.style.backgroundColor = to_hex(hu, 1, 1);
    sv_th.style.left = `${s * 100}%`;
    sv_th.style.top = `${(1 - v) * 100}%`;
    sv_th.style.background = now;
    hue_th.style.left = `${(hu / 360) * 100}%`;
    hue_th.style.background = to_hex(hu, 1, 1);
    hue.setAttribute("aria-valuenow", String(Math.round(hu)));
    sv.setAttribute("aria-valuetext", now);
    prev.style.background = now;
    if (!typed) hex.value = now;
  };
  const set = (typed) => {
    paint(typed);
    onchange(now);
  };

  const drag = (el, at) => {
    el.addEventListener("pointerdown", (e) => {
      e.preventDefault();
      el.focus();
      el.setPointerCapture(e.pointerId);
      const move = (ev) => {
        const r = el.getBoundingClientRect();
        at(Math.min(1, Math.max(0, (ev.clientX - r.left) / r.width)), Math.min(1, Math.max(0, (ev.clientY - r.top) / r.height)));
        set();
      };
      move(e);
      el.onpointermove = move;
      el.onpointerup = () => {
        el.onpointermove = null;
        el.onpointerup = null;
      };
    });
  };
  drag(sv, (x, y) => {
    s = x;
    v = 1 - y;
  });
  drag(hue, (x) => {
    hu = Math.min(359.9, x * 360);
  });
  const step = (e) => (e.shiftKey ? 0.1 : 0.02);
  sv.addEventListener("keydown", (e) => {
    const d = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, 1], ArrowDown: [0, -1] }[e.key];
    if (!d) return;
    e.preventDefault();
    s = Math.min(1, Math.max(0, s + d[0] * step(e)));
    v = Math.min(1, Math.max(0, v + d[1] * step(e)));
    set();
  });
  hue.addEventListener("keydown", (e) => {
    const d = { ArrowLeft: -1, ArrowDown: -1, ArrowRight: 1, ArrowUp: 1 }[e.key];
    if (!d) return;
    e.preventDefault();
    hu = (hu + d * (e.shiftKey ? 20 : 3) + 360) % 360;
    set();
  });
  hex.addEventListener("input", () => {
    let t = hex.value.trim().toLowerCase();
    if (!t.startsWith("#")) t = `#${t}`;
    if (!HEX.test(t)) return;
    [hu, s, v] = of_hex(t);
    set(true);
  });

  const box = h("div", { class: "cpop", role: "dialog", "aria-label": `${title} colour` },
    h("div", { class: "cpop-head" }, h("span", null, title), h("button", { type: "button", class: "cib", "aria-label": "Close", title: "Close", onclick: () => box.close() }, "×")),
    sv, hue,
    h("div", { class: "cpop-row" }, prev, hex),
    used.length
      ? h("div", { class: "cpop-recent" }, h("span", null, "Recent"), used.map((c) => tinted(h("button", { type: "button", title: c, "aria-label": `Use ${c}`, onclick: () => {
        [hu, s, v] = of_hex(c);
        set();
      } }), c)))
      : null,
    h("div", { class: "cpop-row end" },
      h("button", { type: "button", class: "btn small", onclick: () => {
        box.close(true);
        onreset();
      } }, "Use original"),
      h("button", { type: "button", class: "btn small primary", onclick: () => box.close() }, "Done")));

  const outside = (e) => {
    if (!box.contains(e.target) && !anchor.contains(e.target)) box.close();
  };
  const keys = (e) => {
    if (e.key === "Escape") {
      e.preventDefault();
      box.close();
      anchor.focus();
    }
  };
  const place = () => {
    const r = anchor.getBoundingClientRect();
    const w = box.offsetWidth;
    const ht = box.offsetHeight;
    const left = Math.max(8, Math.min(r.left + r.width / 2 - w / 2, window.innerWidth - w - 8));
    const below = r.bottom + 6;
    const top = below + ht > window.innerHeight - 8 ? Math.max(8, r.top - ht - 6) : below;
    box.style.left = `${left}px`;
    box.style.top = `${top}px`;
  };
  const gone = () => box.close();
  box.close = (skip) => {
    if (pop !== box) return;
    pop = null;
    box.remove();
    document.removeEventListener("pointerdown", outside, true);
    document.removeEventListener("keydown", keys, true);
    window.removeEventListener("resize", gone);
    window.removeEventListener("scroll", gone, true);
    if (!skip && HEX.test(now) && now !== start) remember(now);
  };

  pop = box;
  (document.querySelector(".app") || document.body).append(box);
  paint();
  place();
  sv.focus();
  document.addEventListener("pointerdown", outside, true);
  document.addEventListener("keydown", keys, true);
  window.addEventListener("resize", gone);
  window.addEventListener("scroll", gone, true);
}
