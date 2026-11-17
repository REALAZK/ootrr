import { h } from "./dom.js";
import { max_of, dep_text, effective, same, clone } from "./rules.js";
import { post } from "./api.js";
import { state } from "./store.js";
import { tip_button } from "./tooltip.js";

const NS = "http://www.w3.org/2000/svg";

export function reset_icon() {
  const svg = document.createElementNS(NS, "svg");
  svg.setAttribute("viewBox", "0 0 16 16");
  svg.setAttribute("fill", "none");
  svg.setAttribute("stroke", "currentColor");
  svg.setAttribute("stroke-width", "1.8");
  svg.setAttribute("stroke-linecap", "round");
  svg.setAttribute("stroke-linejoin", "round");
  svg.setAttribute("aria-hidden", "true");
  for (const d of ["M3 8a5 5 0 1 0 1.6-3.7", "M2.6 2v3.3h3.3"]) {
    const p = document.createElementNS(NS, "path");
    p.setAttribute("d", d);
    svg.append(p);
  }
  return svg;
}

export function marked(text, terms) {
  const t = (terms || []).filter(Boolean);
  if (!t.length) return [text];
  const esc = t.map((x) => x.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"));
  const re = new RegExp(`(${esc.join("|")})`, "gi");
  return String(text).split(re).map((part, i) => (i % 2 ? h("mark", null, part) : part));
}

function shown(s, v) {
  return s.times ? `${Number(v) / s.times}x` : String(v);
}

function tip_content(s, vals) {
  return () => {
    const kids = [h("div", { class: "tiptext" }, s.tip)];
    const cs = (s.choices || []).filter((c) => c.tip);
    if (s.kind === "choice" && cs.length) kids.push(h("dl", null, cs.map((c) => [h("dt", null, c.label), h("dd", null, c.tip)])));
    if (s.kind === "number") kids.push(h("dl", null, h("dd", null, `From ${shown(s, s.min)} to ${shown(s, max_of(s, vals()))}. Default ${shown(s, s.default)}.`)));
    return kids;
  };
}

function toggle(s, ctx, terms) {
  const box = h("input", { type: "checkbox", id: `c_${s.id}`, onchange: () => ctx.set(s.id, box.checked) });
  return {
    label: h("label", { class: "chk", for: `c_${s.id}` }, box, h("span", null, marked(s.label, terms))),
    body: null,
    sync(v, on) {
      box.checked = v === true;
      box.disabled = !on;
    },
  };
}

function choice(s, ctx, terms) {
  const sel = h(
    "select",
    { id: `c_${s.id}`, onchange: () => ctx.set(s.id, sel.value) },
    s.choices.map((c) => h("option", { value: c.id }, c.label)),
  );
  return {
    label: h("label", { for: `c_${s.id}` }, marked(s.label, terms)),
    body: sel,
    sync(v, on) {
      sel.value = v;
      sel.disabled = !on;
    },
  };
}

function number(s, ctx, terms) {
  const val = h("span", { class: "val" });
  const step = s.step || 1;
  const range = h("input", { type: "range", id: `c_${s.id}`, min: s.min, max: s.max, step: String(step) });
  range.addEventListener("input", () => (val.textContent = shown(s, range.value)));
  range.addEventListener("change", () => ctx.set(s.id, Number.parseInt(range.value, 10)));
  const less = h("button", { type: "button", class: "btn", "aria-label": `Less ${s.label}` }, "−");
  const more = h("button", { type: "button", class: "btn", "aria-label": `More ${s.label}` }, "+");
  let now = s.default;
  let top = s.max;
  less.onclick = () => ctx.set(s.id, Math.max(s.min, now - step));
  more.onclick = () => ctx.set(s.id, Math.min(top, now + step));
  return {
    label: h("label", { for: `c_${s.id}` }, marked(s.label, terms), ": ", val),
    body: h("div", { class: "sl" }, less, range, more),
    sync(v, on, vals) {
      const m = max_of(s, vals);
      now = v;
      top = m;
      range.max = m;
      range.value = v;
      val.textContent = shown(s, v);
      range.disabled = !on;
      less.disabled = !on || v <= s.min;
      more.disabled = !on || v >= m;
    },
  };
}

function multi(s, ctx, terms) {
  const boxes = new Map();
  const count = h("span", null);
  const grid = h(
    "div",
    { class: "cgrid" },
    s.choices.map((c) => {
      const b = h("input", { type: "checkbox", value: c.id });
      boxes.set(c.id, b);
      b.addEventListener("change", () => ctx.set(s.id, [...boxes].filter(([, x]) => x.checked).map(([k]) => k)));
      return h("label", { class: "chk", title: c.tip || null }, b, h("span", null, marked(c.label, terms)));
    }),
  );
  const clear = h("button", { type: "button", class: "btn small", onclick: () => ctx.set(s.id, []) }, "Clear");
  return {
    label: h("span", null, marked(s.label, terms)),
    after: h("span", null, count, clear),
    body: grid,
    sync(v, on) {
      for (const [k, b] of boxes) {
        b.checked = v.includes(k);
        b.disabled = !on;
      }
      clear.disabled = !on || v.length === 0;
      count.textContent = `${v.length} of ${s.choices.length} selected`;
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

function list(s, ctx, terms) {
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
    label: h("span", null, marked(s.label, terms)),
    body: x ? x.el : h("div", null, ta, hint),
    sync(v, on) {
      if (document.activeElement !== ta) ta.value = v.join("\n");
      ta.disabled = !on;
      hint.textContent = `Up to ${s.max_items} entries. Letters, digits, spaces and _ . : ' ( ) / - only.`;
      if (x) x.sync(v);
    },
  };
}

const KINDS = { toggle, choice, number, multi, list };

export const is_wide = (s) => s.kind === "multi" || s.kind === "list";

export function make_row(schema, s, ctx, opts = {}) {
  let vals = {};
  const c = KINDS[s.kind](s, ctx, opts.terms);
  const rst = h("button", { type: "button", class: "dot", title: "Changed from default. Click to reset.", "aria-label": `Reset ${s.label} to default`, hidden: true }, reset_icon());
  rst.onclick = () => ctx.set(s.id, clone(s.default));
  const why = h("div", { class: "why" });
  const head = h("div", { class: "wl" }, c.label, tip_button(s.label, tip_content(s, () => vals)), rst);
  const top = c.after ? h("div", { class: "cbar" }, head, c.after) : head;
  const el = h("div", { class: "w", "data-id": s.id }, top, c.body, why, opts.note ? h("div", { class: "hitnote" }, opts.note) : null);
  return {
    s,
    el,
    sync(eff, on) {
      vals = eff;
      const locked = Boolean(s.unsupported) && same(eff[s.id], s.default);
      c.sync(eff[s.id], on && !locked, eff);
      el.classList.toggle("off", !on || locked);
      el.hidden = !on && s.kind === "number" && !opts.keep;
      rst.hidden = !on || same(eff[s.id], s.default);
      if (!on) why.textContent = `Needs ${dep_text(schema, s)}`;
      else if (locked) why.textContent = "Not implemented yet. Only the default works.";
      else if (s.unsupported) why.textContent = "Not implemented yet. This value stops the build. Reset it.";
      else why.textContent = "";
    },
  };
}
