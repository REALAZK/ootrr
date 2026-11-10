import { h } from "./dom.js";
import { max_of, dep_text } from "./rules.js";

function head(s) {
  return h("div", { class: "head" }, h("span", { class: "title" }, s.label), h("span", { class: "badge" }, s.tag));
}

function toggle(s, ctx) {
  const box = h("input", { type: "checkbox", id: `c_${s.id}`, onchange: () => ctx.set(s.id, box.checked) });
  const lab = h("label", { class: "switch", for: `c_${s.id}` }, box, h("span", null, s.tip));
  return {
    el: lab,
    tip: null,
    sync(v, on) {
      box.checked = v === true;
      box.disabled = !on;
    },
  };
}

function choice(s, ctx) {
  const sel = h(
    "select",
    { id: `c_${s.id}`, "aria-label": s.label, onchange: () => ctx.set(s.id, sel.value) },
    s.choices.map((c) => h("option", { value: c.id }, c.label)),
  );
  const hint = h("div", { class: "hint" });
  return {
    el: h("div", null, sel, hint),
    tip: s.tip,
    sync(v, on) {
      sel.value = v;
      sel.disabled = !on;
      const c = s.choices.find((x) => x.id === v);
      hint.textContent = (c && c.tip) || "";
    },
  };
}

function number(s, ctx) {
  const inp = h("input", {
    type: "number",
    id: `c_${s.id}`,
    "aria-label": s.label,
    min: s.min,
    max: s.max,
    step: "1",
    inputmode: "numeric",
    onchange: () => {
      const n = Number.parseInt(inp.value, 10);
      ctx.set(s.id, Number.isFinite(n) ? n : s.default);
    },
  });
  const hint = h("div", { class: "hint" });
  return {
    el: h("div", null, inp, hint),
    tip: s.tip,
    sync(v, on, vals) {
      const m = max_of(s, vals);
      inp.max = m;
      inp.value = v;
      inp.disabled = !on;
      hint.textContent = `From ${s.min} to ${m}`;
    },
  };
}

function multi(s, ctx) {
  const boxes = new Map();
  const count = h("span", null);
  const chips = h(
    "div",
    { class: "chips" },
    s.choices.map((c) => {
      const b = h("input", { type: "checkbox", value: c.id });
      boxes.set(c.id, b);
      const lab = h("label", { class: "chip", title: c.tip || null }, b, c.label);
      b.addEventListener("change", () => {
        const now = [...boxes].filter(([, x]) => x.checked).map(([k]) => k);
        ctx.set(s.id, now);
      });
      return lab;
    }),
  );
  const clear = h("button", { type: "button", class: "btn small", onclick: () => ctx.set(s.id, []) }, "Clear");
  return {
    el: h("details", null, h("summary", null, count), chips, h("div", { class: "row", }, clear)),
    tip: s.tip,
    sync(v, on) {
      for (const [k, b] of boxes) {
        b.checked = v.includes(k);
        b.disabled = !on;
        b.parentElement.classList.toggle("on", b.checked);
      }
      clear.disabled = !on || v.length === 0;
      count.textContent = v.length === 0 ? "None selected" : `${v.length} of ${s.choices.length} selected`;
    },
  };
}

function list(s, ctx) {
  const ta = h("textarea", {
    id: `c_${s.id}`,
    "aria-label": s.label,
    spellcheck: "false",
    rows: "4",
    placeholder: "One location id per line",
    onchange: () => {
      const items = ta.value.split("\n").map((x) => x.trim()).filter((x) => x.length > 0);
      ctx.set(s.id, items);
    },
  });
  const hint = h("div", { class: "hint" });
  return {
    el: h("div", null, ta, hint),
    tip: s.tip,
    sync(v, on) {
      if (document.activeElement !== ta) ta.value = v.join("\n");
      ta.disabled = !on;
      hint.textContent = `Up to ${s.max_items} entries. Letters, digits, spaces and _ . : ' ( ) / - only.`;
    },
  };
}

const KINDS = { toggle, choice, number, multi, list };

export function make_row(schema, s, ctx) {
  const c = KINDS[s.kind](s, ctx);
  const tip = c.tip ? h("div", { class: "hint" }, c.tip) : null;
  const note = h("div", { class: "depnote" });
  const wide = s.kind === "multi" || s.kind === "list";
  const el = h("div", { class: `setting${wide ? " wide" : ""}`, "data-id": s.id }, head(s), tip, c.el, note);
  return {
    s,
    el,
    sync(vals, on) {
      c.sync(vals[s.id], on, vals);
      el.classList.toggle("off", !on);
      note.textContent = on ? "" : `Switched off. It applies when ${dep_text(schema, s)}.`;
    },
  };
}
