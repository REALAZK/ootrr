import { h } from "./dom.js";
import { max_of, dep_text, effective } from "./rules.js";
import { post } from "./api.js";
import { state } from "./store.js";

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

function exclusions(s, ctx, ta, hint) {
  const find = h("input", { type: "text", placeholder: "Search checks, then drag or click one in", autocomplete: "off", spellcheck: "false" });
  const pool = h("ul", { class: "excl-pool" });
  const chosen = h("ul", { class: "excl-chosen" });
  const msg = h("div", { class: "hint" });
  let all = [];
  let have = [];
  const commit = (v) => {
    have = v;
    ctx.set(s.id, v);
  };
  const add = (n) => {
    if (have.some((x) => x.toLowerCase() === n.toLowerCase())) return;
    if (have.length >= s.max_items) return void (msg.textContent = `At most ${s.max_items} locations.`);
    msg.textContent = "";
    commit([...have, n]);
  };
  const draw = () => {
    const q = find.value.trim().toLowerCase();
    const rows = all.filter((n) => !have.includes(n) && (!q || n.toLowerCase().includes(q))).slice(0, 60);
    pool.replaceChildren(
      ...rows.map((n) => {
        const li = h("li", { draggable: "true", title: "Drag into the excluded list" }, n);
        li.ondragstart = (e) => e.dataTransfer.setData("text/plain", n);
        li.onclick = () => add(n);
        return li;
      }),
    );
    chosen.replaceChildren(
      ...have.map((n) => {
        const b = h("button", { type: "button", class: "btn quiet small", "aria-label": `Remove ${n}` }, "Remove");
        b.onclick = () => commit(have.filter((x) => x !== n));
        return h("li", null, h("span", null, n), b);
      }),
    );
  };
  const zone = h("div", { class: "excl-zone" }, chosen);
  zone.ondragover = (e) => {
    e.preventDefault();
    zone.classList.add("over");
  };
  zone.ondragleave = () => zone.classList.remove("over");
  zone.ondrop = (e) => {
    e.preventDefault();
    zone.classList.remove("over");
    const n = e.dataTransfer.getData("text/plain").trim();
    if (n) add(n);
  };
  find.oninput = draw;
  find.onfocus = async () => {
    if (all.length) return;
    try {
      const r = await post("/api/plando/options", { settings: effective(state.schema, state.vals) });
      all = r.locations;
      draw();
    } catch (e) {
      msg.textContent = `The location list could not be loaded. ${e.message}`;
    }
  };
  const det = h("details", null, h("summary", null, "Edit as text"), ta, hint);
  return {
    el: h("div", { class: "excl" }, find, pool, h("div", { class: "hint" }, "Excluded checks can only hold items the seed does not need. Too many and the seed is refused."), zone, msg, det),
    sync(v) {
      have = v.slice();
      draw();
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
  const x = s.id === "disabled_locations" ? exclusions(s, ctx, ta, hint) : null;
  return {
    el: x ? x.el : h("div", null, ta, hint),
    tip: s.tip,
    sync(v, on) {
      if (document.activeElement !== ta) ta.value = v.join("\n");
      ta.disabled = !on;
      hint.textContent = `Up to ${s.max_items} entries. Letters, digits, spaces and _ . : ' ( ) / - only.`;
      if (x) x.sync(v);
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
