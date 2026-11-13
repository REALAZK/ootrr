import { h } from "./dom.js";

const KEY = "ootrr.sound";
const PICK = ".btn, .step, .tab, .preset, .chip, .ents button, summary";

const VOICES = {
  select: [[1568, 0], [2093, 0.055]],
  enter: [[523, 0], [659, 0.07], [784, 0.14], [1047, 0.21]],
  equip: [[784, 0], [1175, 0.06]],
  unequip: [[1175, 0], [784, 0.06]],
  leave: [[1047, 0], [784, 0.08], [523, 0.16]],
};

let ctx = null;
let master = null;
let on = true;
let btn = null;

function ensure() {
  if (ctx) return ctx;
  const AC = window.AudioContext || window.webkitAudioContext;
  if (!AC) return null;
  ctx = new AC({ latencyHint: "interactive" });
  master = ctx.createGain();
  master.gain.value = 0.3;
  master.connect(ctx.destination);
  return ctx;
}

function note(freq, at, len, type, vol) {
  const g = ctx.createGain();
  g.gain.setValueAtTime(0.0001, at);
  g.gain.exponentialRampToValueAtTime(vol, at + 0.006);
  g.gain.exponentialRampToValueAtTime(0.0001, at + len);
  g.connect(master);
  const o = ctx.createOscillator();
  o.type = type;
  o.frequency.value = freq;
  o.connect(g);
  o.start(at);
  o.stop(at + len + 0.03);
}

function ready() {
  if (ctx && ctx.state === "running") return ctx;
  const act = navigator.userActivation;
  if (act && !act.isActive) return null;
  const c = ensure();
  if (c && c.state === "suspended") c.resume();
  return c;
}

export function play(name) {
  if (!on || !VOICES[name]) return;
  const c = ready();
  if (!c) return;
  const t = c.currentTime;
  for (const [freq, off] of VOICES[name]) {
    note(freq, t + off, 0.22, "sine", 0.5);
    note(freq * 2, t + off, 0.18, "triangle", 0.12);
  }
}

function target(e) {
  const el = e.target instanceof Element ? e.target.closest(PICK) : null;
  if (!el || el.id === "sound-pick") return null;
  if (el.disabled || el.getAttribute("aria-disabled") === "true") return null;
  if (el.closest("label")) return null;
  return el;
}

function paint() {
  btn.setAttribute("aria-pressed", String(on));
  btn.textContent = on ? "Sound on" : "Sound off";
}

function remember() {
  try {
    localStorage.setItem(KEY, on ? "1" : "0");
  } catch {
    return;
  }
}

function load() {
  try {
    return localStorage.getItem(KEY) !== "0";
  } catch {
    return true;
  }
}

export function start() {
  on = load();
  btn = h("button", {
    class: "btn small",
    id: "sound-pick",
    type: "button",
    "aria-label": "Interface sounds",
    onclick: () => {
      on = !on;
      remember();
      paint();
      if (on) play("equip");
    },
  });
  paint();
  const box = document.querySelector(".top-in");
  if (box) box.append(btn);
  document.addEventListener("click", (e) => {
    const el = target(e);
    if (el) play(el.classList.contains("primary") ? "enter" : "select");
  });
  document.addEventListener("change", (e) => {
    const t = e.target;
    if (t instanceof HTMLInputElement && t.type === "checkbox") play(t.checked ? "equip" : "unequip");
  });
}
