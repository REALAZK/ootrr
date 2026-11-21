import { h, fill, toast } from "./dom.js";
import { post } from "./api.js";
import { state, touch, color_groups } from "./store.js";
import { open_picker, close_picker } from "./color_pop.js";

const SWATCH = {
  kokiri: "#2f8f3a", kokiri_pants: "#3d7a33", kokiri_shirt: "#d8d1bd", kokiri_belt: "#6b4a2b", feathers: "#e6dfcf",
  goron: "#b3262c", goron_trim: "#d4a634", goron_belt: "#6b4a2b", goron_beads: "#c9b07a",
  zora: "#2a4fb0", zora_belt: "#8a3c8e", zora_shirt: "#d4d8de", zora_scales: "#3d8fb8",
  boots: "#5b3f26", gloves: "#7a5534", leather_trim: "#9c2f25", kokiri_sword: "#b03040", master_sword: "#4a3fa8",
  biggoron_sword: "#8c8f99", deku_shield: "#b8562e", hylian_shield: "#2d55a8", mirror_shield: "#b42a2a",
  boomerang: "#c0302e", gauntlets: "#2a8fc0", hair: "#e0b84a", navi: "#dff3ff", hearts: "#e8364a",
};

const ICONS = {
  die: "M4 7a3 3 0 0 1 3-3h10a3 3 0 0 1 3 3v10a3 3 0 0 1-3 3H7a3 3 0 0 1-3-3z M9 9h.01 M15 15h.01 M15 9h.01 M9 15h.01",
  reset: "M4 12a8 8 0 1 0 2.4-5.7 M4 4v4h4",
  lock: "M7 11V8a5 5 0 0 1 10 0v3 M5 11h14v9H5z",
  open: "M7 11V8a5 5 0 0 1 9.6-2 M5 11h14v9H5z",
};

const HEX = /^#[0-9a-f]{6}$/;
let open = null;

function tinted(el, c) {
  el.style.background = c;
  return el;
}

function icon(name) {
  const s = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  s.setAttribute("viewBox", "0 0 24 24");
  s.setAttribute("aria-hidden", "true");
  const p = document.createElementNS("http://www.w3.org/2000/svg", "path");
  p.setAttribute("d", ICONS[name]);
  s.append(p);
  return s;
}

function ib(name, label, onclick, pressed) {
  return h("button", { type: "button", class: "cib", title: label, "aria-label": label, "aria-pressed": pressed === undefined ? null : String(pressed), onclick }, icon(name));
}

function shown(id) {
  const v = state.cosmetic.parts[id];
  return HEX.test(v || "") ? v : SWATCH[id] || "#808080";
}

function said(id) {
  const v = state.cosmetic.parts[id];
  if (HEX.test(v || "")) return v;
  return v === "random" ? "new colour each build" : "original";
}

async function rolled(style) {
  const r = await post("/api/colors/roll", { style });
  return r.colors || {};
}

export function colour_editor() {
  const cos = state.cosmetic;
  cos.parts = cos.parts || {};
  cos.locks = cos.locks || {};
  const box = h("div", { class: "ced", id: "f_colors" });
  const groups = [...color_groups()];
  if (!open) open = new Set(groups.map(([g]) => g));
  let busy = false;

  const changed = () => {
    close_picker();
    touch();
    draw();
  };
  const unlocked = (ps) => ps.filter((p) => !cos.locks[p.id]);
  const roll_into = async (ps, style) => {
    if (busy) return;
    busy = true;
    try {
      const c = await rolled(style);
      for (const p of ps) if (c[p.id]) cos.parts[p.id] = c[p.id];
      if (ps.length > 1) cos.style = style;
      changed();
    } catch (e) {
      toast(e.message);
    } finally {
      busy = false;
    }
  };

  const row = (g, p) => {
    const lk = !!cos.locks[p.id];
    const text = h("span", { class: `chex${said(p.id) === "original" ? "" : " on"}` }, said(p.id));
    const sw = tinted(h("button", { type: "button", class: "cswatch", "aria-label": `${g} ${p.label} colour, ${said(p.id)}`, "aria-haspopup": "dialog" }), shown(p.id));
    sw.addEventListener("click", () => open_picker(sw, {
      title: `${g} · ${p.label}`,
      value: shown(p.id),
      onchange: (hex) => {
        cos.parts[p.id] = hex;
        text.textContent = said(p.id);
        text.className = "chex on";
        sw.style.background = hex;
        sw.setAttribute("aria-label", `${g} ${p.label} colour, ${hex}`);
        for (const c of box.querySelectorAll(`[data-chip="${p.id}"]`)) c.style.background = hex;
        touch();
      },
      onreset: () => {
        delete cos.parts[p.id];
        changed();
      },
    }));
    return h("div", { class: `crow${lk ? " lk" : ""}` }, sw,
      h("div", { class: "cname" }, h("span", null, p.label), text),
      h("div", { class: "cacts" },
        ib("die", `Randomize ${g} ${p.label}`, () => roll_into([p], "random")),
        ib("reset", `Reset ${g} ${p.label}`, () => {
          delete cos.parts[p.id];
          changed();
        }),
        ib(lk ? "lock" : "open", lk ? `Unlock ${g} ${p.label}` : `Lock ${g} ${p.label}`, () => {
          if (lk) delete cos.locks[p.id];
          else cos.locks[p.id] = true;
          changed();
        }, lk)));
  };

  const fold = ([g, ps]) => {
    const n = ps.filter((p) => cos.locks[p.id]).length;
    const all = n === ps.length;
    const d = h("details", { class: "cfold" },
      h("summary", null,
        h("span", { class: "cgname" }, g),
        h("span", { class: "cchips", "aria-hidden": "true" }, ps.map((p) => tinted(h("i", { "data-chip": p.id }), shown(p.id)))),
        h("small", null, n ? `${n}/${ps.length} locked` : `${ps.length} ${ps.length === 1 ? "part" : "parts"}`),
        h("span", { class: "csp" }),
        ib(all ? "lock" : "open", all ? `Unlock ${g}` : `Lock ${g}`, (e) => {
          e.preventDefault();
          for (const p of ps) {
            if (all) delete cos.locks[p.id];
            else cos.locks[p.id] = true;
          }
          changed();
        }, all)),
      h("div", { class: "crows" }, ps.map((p) => row(g, p))));
    d.open = open.has(g);
    d.addEventListener("toggle", () => {
      if (d.open) open.add(g);
      else open.delete(g);
    });
    return d;
  };

  const draw = () => {
    const ps = groups.flatMap(([, x]) => x);
    const n = ps.filter((p) => cos.locks[p.id]).length;
    const style = h("select", { "aria-label": "Colours drawn at each build", disabled: !cos.reroll },
      h("option", { value: "random" }, "A random colour for each part"),
      h("option", { value: "shared" }, "Shades of one random colour"));
    style.value = cos.style === "shared" ? "shared" : "random";
    style.addEventListener("change", () => {
      cos.style = style.value;
      touch();
    });
    const re = h("input", { type: "checkbox", id: "f_reroll", checked: !!cos.reroll });
    re.addEventListener("change", () => {
      cos.reroll = re.checked;
      changed();
    });
    fill(box,
      h("div", { class: "cedbar" },
        h("button", { type: "button", class: "btn", onclick: () => roll_into(unlocked(ps), "random") }, "Randomize unlocked"),
        h("button", { type: "button", class: "btn", onclick: () => roll_into(unlocked(ps), "shared") }, "Shades of one colour"),
        h("button", { type: "button", class: "btn", onclick: () => {
          for (const p of unlocked(ps)) delete cos.parts[p.id];
          changed();
        } }, "Reset unlocked"),
        h("span", { class: "csp" }),
        h("span", { class: "hint" }, `${n} locked`),
        h("button", { type: "button", class: "btn", onclick: () => {
          if (n === ps.length) cos.locks = {};
          else for (const p of ps) cos.locks[p.id] = true;
          changed();
        } }, n === ps.length ? "Unlock all" : "Lock all")),
      h("div", { class: "cfolds" }, groups.map(fold)),
      h("div", { class: "cedbar" },
        h("label", { class: "chk", for: "f_reroll" }, re, h("span", null, "Re-roll unlocked parts on every build")),
        cos.reroll ? style : null),
      h("p", { class: "hint" }, cos.reroll
        ? "Locked parts keep the colour shown. Every other part gets a new colour at each build."
        : "The mod gets exactly the colours shown. Parts marked original keep the game's own colour."));
  };
  draw();
  return box;
}
