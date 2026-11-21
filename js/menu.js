import { $, h, fill, notice, copy, debounce } from "./dom.js";
import { post } from "./api.js";
import { state, touch, on_change } from "./store.js";
import { defaults, effective, enabled, from_preset, match_preset, changed, with_value, clone, same, dep_text } from "./rules.js";
import { make_row, is_wide } from "./controls.js";
import { FIELDS, field_el, gen_line, gen_missing, page_fields } from "./config.js";
import { build_index, search, grouped } from "./search.js";
import { generate, busy, on_run, render_seed, set_pages } from "./run.js";
import { panel as plando_panel } from "./plando.js";
import { live, refresh_string } from "./strings.js";
import { register, set_first, on_show, go, show, current_page } from "./nav.js";
import { tip_button } from "./tooltip.js";

let pages = [];
let index = [];
let rows = [];
let query = "";
let gen_draw = () => {};

const ctx = {
  set(id, v) {
    state.vals = with_value(state.schema, state.vals, id, v);
    touch();
    sync_rows();
  },
};

function sync_rows() {
  const eff = effective(state.schema, state.vals);
  for (const r of rows) r.sync(eff, enabled(r.s, eff));
}

export function head(title, sub) {
  return h("div", { class: "mhead" }, h("h3", null, title), sub ? h("span", null, sub) : null);
}

export function section(label, kids, full) {
  return h("div", { class: `msec${full ? " full" : ""}` }, h("h4", null, label), kids);
}

function settings_page(p) {
  const by_id = new Map(state.schema.settings.map((s) => [s.id, s]));
  let stubs = 0;
  const secs = p.sections.map((sec) => {
    const items = sec.items || sec.ids.map((id) => ({ id }));
    if (!items.length) return null;
    const ss = sec.ids.map((id) => by_id.get(id)).filter(Boolean);
    const kids = items.map((it) => {
      if (it.stub) {
        stubs++;
        return stub_row(it);
      }
      const s = by_id.get(it.id);
      if (!s) return null;
      if (it.view) {
        const r = view_row(s, it.view);
        rows.push(r);
        return r.el;
      }
      const r = make_row(state.schema, s, ctx);
      if (it.only) for (const o of r.el.querySelectorAll("select option")) o.disabled = !it.only.includes(o.value);
      rows.push(r);
      return r.el;
    });
    return section(sec.label, kids, ss.some(is_wide));
  });
  const n = p.sections.reduce((a, s) => a + s.ids.length, 0);
  const sub = `${p.heading} · ${n} setting${n === 1 ? "" : "s"}${stubs ? ` · ${stubs} not supported yet` : ""}`;
  const warn = p.warning ? h("p", { class: "notice warn mwarn" }, p.warning) : null;
  if (!secs.length) return [head(p.label), warn, h("p", { class: "mempty" }, "Nothing here yet.")];
  return [head(p.label), warn, h("div", { class: "mgrid" }, secs)];
}

// An option's needs: "id" (a toggle that must be on), "id=value" or "id!=value".
function needs_met(n, eff) {
  const ne = n.indexOf("!=");
  if (ne >= 0) return eff[n.slice(0, ne)] !== n.slice(ne + 2);
  const eq = n.indexOf("=");
  if (eq >= 0) return eff[n.slice(0, eq)] === n.slice(eq + 1);
  return eff[n] === true;
}

function view_row(s, v) {
  const val = (x) => (s.kind === "toggle" ? x === "true" : x);
  if (v.kind === "check") return check_row(s, v, val);
  if (v.kind === "steps") return steps_row(s, v, val);
  if (v.kind === "number") return number_row(s, v);
  const shown = { ...s, label: v.label, tip: v.tip, kind: "choice", choices: v.opts.map((o, i) => ({ id: o.ours || `soh_${i}`, label: o.label })) };
  const vctx = { set: (id, x) => ctx.set(id, typeof x === "string" ? val(x) : x) };
  const r = make_row(state.schema, shown, vctx);
  const opts = r.el.querySelectorAll("select option");
  opts.forEach((o, i) => (o.disabled = !v.opts[i].live));
  if (!v.opts.some((o) => o.needs)) return r;
  const sync = r.sync;
  r.sync = (eff, ok) => {
    sync(eff, ok);
    opts.forEach((o, i) => {
      const n = v.opts[i].needs;
      o.disabled = !v.opts[i].live || (n !== "" && !needs_met(n, eff));
    });
  };
  return r;
}

function check_row(s, v, val) {
  const [off, on] = [v.opts[0], v.opts[1]];
  const box = h("input", { type: "checkbox", id: `c_${s.id}` });
  box.onchange = () => ctx.set(s.id, val(box.checked ? on.ours : off.ours));
  const why = h("div", { class: "why" });
  const el = h("div", { class: "w", "data-id": s.id }, h("div", { class: "wl" }, h("label", { class: "chk", for: `c_${s.id}` }, box, h("span", null, v.label)), tip_of(v.label, v.tip)), why);
  return {
    s,
    el,
    sync(eff, ok) {
      box.checked = same(eff[s.id], val(on.ours));
      const can = ok && (box.checked ? off.live : on.live);
      box.disabled = !can;
      el.classList.toggle("off", !can);
      why.textContent = ok ? "" : `Needs ${dep_text(state.schema, s)}`;
    },
  };
}

function steps_row(s, v, val) {
  const pts = v.opts.map((o) => ({ n: Number(o.label), v: val(o.ours), live: o.live }));
  const shown = h("span", { class: "val" });
  const range = h("input", { type: "range", id: `c_${s.id}`, min: v.min, max: v.max, step: "1" });
  const nearest = (n) => pts.filter((p) => p.live).reduce((a, p) => (Math.abs(p.n - n) < Math.abs(a.n - n) ? p : a));
  range.addEventListener("input", () => (shown.textContent = String(nearest(Number(range.value)).n)));
  range.addEventListener("change", () => ctx.set(s.id, nearest(Number(range.value)).v));
  let now = pts[0];
  const live = pts.filter((p) => p.live).sort((a, b) => a.n - b.n);
  const less = h("button", { type: "button", class: "btn", "aria-label": `Less ${v.label}` }, "\u2212");
  const more = h("button", { type: "button", class: "btn", "aria-label": `More ${v.label}` }, "+");
  less.onclick = () => {
    const p = [...live].reverse().find((x) => x.n < now.n);
    if (p) ctx.set(s.id, p.v);
  };
  more.onclick = () => {
    const p = live.find((x) => x.n > now.n);
    if (p) ctx.set(s.id, p.v);
  };
  const why = h("div", { class: "why" });
  const el = h("div", { class: "w", "data-id": s.id }, h("div", { class: "wl" }, h("label", { for: `c_${s.id}` }, v.label, ": ", shown), tip_of(v.label, v.tip)), h("div", { class: "sl" }, less, range, more), why);
  return {
    s,
    el,
    sync(eff, ok) {
      const p = pts.find((x) => same(x.v, eff[s.id])) || pts[0];
      now = p;
      range.value = String(p.n);
      shown.textContent = String(p.n);
      range.disabled = !ok;
      less.disabled = !ok || !live.some((x) => x.n < p.n);
      more.disabled = !ok || !live.some((x) => x.n > p.n);
      el.classList.toggle("off", !ok);
      why.textContent = ok ? "" : `Needs ${dep_text(state.schema, s)}`;
    },
  };
}

function number_row(s, v) {
  const r = make_row(state.schema, { ...s, label: v.label, tip: v.tip, min: v.min, max: v.max }, ctx, { keep: true });
  if (v.live) return r;
  const sync = r.sync;
  r.sync = (eff, ok) => {
    sync(eff, ok);
    r.el.querySelectorAll("input, button").forEach((x) => (x.disabled = true));
    r.el.classList.add("off");
  };
  return r;
}

function tip_of(label, tip) {
  return tip ? tip_button(label, () => [h("div", { class: "tiptext" }, tip)]) : null;
}

function stub_row(it) {
  const label = it.stub;
  const tip = tip_of(label, it.tip);
  if (it.kind === "choice") {
    const sel = h("select", { disabled: true, "aria-label": label }, it.choices.map((c, i) => h("option", { value: String(i) }, c)));
    sel.value = String(it.value);
    return h("div", { class: "w off stub" }, h("div", { class: "wl" }, h("label", null, label), tip), sel);
  }
  if (it.kind === "number") {
    const range = h("input", { type: "range", min: it.min, max: it.max, value: it.value, disabled: true, "aria-label": label });
    const sl = h("div", { class: "sl" }, h("button", { type: "button", class: "btn", disabled: true }, "−"), range, h("button", { type: "button", class: "btn", disabled: true }, "+"));
    return h("div", { class: "w off stub" }, h("div", { class: "wl" }, h("label", null, label, ": ", h("span", { class: "val" }, String(it.value))), tip), sl);
  }
  return h("div", { class: "w off stub" }, h("div", { class: "wl" }, h("label", { class: "chk" }, h("input", { type: "checkbox", disabled: true }), h("span", null, label)), tip));
}

function config_page(id, title, sub) {
  const secs = [];
  for (const [label, fs] of page_fields(id)) {
    const kids = fs.map((f) => field_el(f));
    if (label === "Seed") {
      const g = gen_line();
      gen_draw = g.draw;
      kids.push(g.el);
    }
    secs.push(section(label, kids, id === "cosmetics" && label === "Colours"));
  }
  return [head(title), h("div", { class: "mgrid" }, secs)];
}

function plando_page() {
  const p = plando_panel();
  p.open = true;
  return [head("Plandomizer"), p];
}

function results() {
  const hits = search(index, query);
  if (!hits.length) return [head("No results"), h("p", { class: "mempty" }, `Nothing matches “${query}”. Try fewer words, or a setting's name.`)];
  const groups = grouped(hits);
  const secs = groups.map((xs) => {
    const p = xs[0].page;
    const crumb = h("button", { type: "button", class: "crumb", onclick: () => jump(p.id, xs[0].id) }, p.label);
    const kids = xs.map((x) => {
      const note = [x.section, x.note].filter(Boolean).join(" · ");
      if (x.kind === "field") {
        const el = field_el(x.f, x.terms);
        el.append(h("div", { class: "hitnote" }, note));
        return el;
      }
      if (x.kind === "stub") {
        const el = stub_row(x.it);
        el.append(h("div", { class: "hitnote" }, note));
        return el;
      }
      if (x.view) {
        const r = view_row(x.s, x.view);
        r.el.append(h("div", { class: "hitnote" }, note));
        rows.push(r);
        return r.el;
      }
      const r = make_row(state.schema, x.s, ctx, { keep: true, terms: x.terms, note });
      rows.push(r);
      return r.el;
    });
    return h("div", { class: `msec${xs.some((x) => x.s && is_wide(x.s)) ? " full" : ""}` }, h("h4", null, `${p.heading} › `, crumb), kids);
  });
  return [head(`${hits.length} result${hits.length === 1 ? "" : "s"}`, `for “${query}” across ${groups.length} page${groups.length === 1 ? "" : "s"}`), h("div", { class: "mgrid" }, secs)];
}

function jump(page, id) {
  set_query("");
  go(page);
  requestAnimationFrame(() => {
    const el = document.querySelector(`[data-id="${CSS.escape(id)}"]`);
    if (!el) return;
    el.scrollIntoView({ block: "center" });
    el.classList.add("flash");
    setTimeout(() => el.classList.remove("flash"), 900);
  });
}

function screen(fn) {
  return () => {
    rows = [];
    gen_draw = () => {};
    const kids = query.trim() ? results() : fn();
    sync_rows();
    return kids;
  };
}

function set_query(v) {
  query = v;
  const f = $("#find");
  if (f && f.value !== v) f.value = v;
}

function nav_items() {
  const out = [{ heading: "Configuration", id: "config", label: "Configuration" }];
  if (state.result || busy()) out.push({ heading: "Configuration", id: "seed", label: "Seed" });
  for (const p of pages) {
    out.push({ heading: p.heading, id: p.id, label: p.label });
    if (p.id === "excluded") out.push({ heading: p.heading, id: "plando", label: "Plandomizer" });
    if (p.id === "hints") out.push({ heading: p.heading, id: "cosmetics", label: "Cosmetics" });
  }
  if (!out.some((x) => x.id === "cosmetics")) out.push({ heading: "Gameplay", id: "cosmetics", label: "Cosmetics" });
  if (!out.some((x) => x.id === "plando")) out.push({ heading: "Randomizer", id: "plando", label: "Plandomizer" });
  return out;
}

function draw_nav() {
  const box = $("#side .mnav");
  const counts = query.trim() ? search(index, query).reduce((m, x) => m.set(x.page.id, (m.get(x.page.id) || 0) + 1), new Map()) : null;
  const now = current_page();
  const kids = [];
  let last = "";
  for (const it of nav_items()) {
    if (it.heading !== last) {
      kids.push(h("div", { class: "mcat" }, it.heading));
      last = it.heading;
    }
    const n = counts ? counts.get(it.id) || 0 : null;
    kids.push(
      h(
        "button",
        { type: "button", class: `mpage${counts && !n ? " dim" : ""}`, "aria-current": !counts && it.id === now ? "page" : null, onclick: () => {
          set_query("");
          go(it.id);
        } },
        it.label,
        counts ? h("span", { class: `n${n ? " hit" : ""}` }, String(n)) : null,
      ),
    );
  }
  fill(box, kids);
}

function search_icon() {
  const ns = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(ns, "svg");
  for (const [k, v] of Object.entries({ viewBox: "0 0 14 14", fill: "none", stroke: "currentColor", "stroke-width": "2", "aria-hidden": "true" })) svg.setAttribute(k, v);
  const c = document.createElementNS(ns, "circle");
  c.setAttribute("cx", "6");
  c.setAttribute("cy", "6");
  c.setAttribute("r", "4.5");
  const p = document.createElementNS(ns, "path");
  p.setAttribute("d", "M9.5 9.5 13 13");
  svg.append(c, p);
  return svg;
}

function sidebar() {
  const find = h("input", { type: "search", id: "find", placeholder: "Search settings   /", "aria-label": "Search settings", autocomplete: "off", spellcheck: "false" });
  const later = debounce(() => show(), 120);
  find.addEventListener("input", () => {
    query = find.value;
    later();
  });
  find.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && find.value) {
      set_query("");
      show();
    }
  });
  const reset = h("button", { type: "button", class: "btn small", id: "reset-all", title: "Put every randomizer setting back to its default" }, "Reset all");
  reset.onclick = () => {
    state.vals = clone(defaults(state.schema));
    touch();
    show();
  };
  fill(
    $("#side"),
    h("div", { class: "msearch" }, h("div", { class: "mfind" }, search_icon(), find)),
    h("nav", { class: "mnav", "aria-label": "Settings pages" }),
    h("div", { class: "mfoot" }, reset, h("span", { id: "sfx-slot" }), h("a", { class: "btn small", href: "https://github.com/ootrando/ootrr", rel: "noopener" }, "Source")),
  );
  document.addEventListener("keydown", (e) => {
    const t = document.activeElement && document.activeElement.tagName;
    if (e.key === "/" && !/^(INPUT|TEXTAREA|SELECT)$/.test(t)) {
      e.preventDefault();
      find.focus();
    }
  });
}

let top = {};

function topbar() {
  const schema = state.schema;
  const preset = h("select", { id: "m_preset" });
  preset.addEventListener("change", () => {
    const p = schema.presets.find((x) => x.id === preset.value);
    if (!p) return;
    state.vals = from_preset(schema, p);
    touch();
    show();
  });
  const changed_n = h("span", { class: "mchanged" });
  const str = h("input", { type: "text", id: "m_string", readonly: true, "aria-label": "Settings string", spellcheck: "false" });
  const copy_btn = h("button", { type: "button", class: "btn", onclick: () => live.text && copy(live.text) }, "Copy");
  const imp_btn = h("button", { type: "button", class: "btn", "aria-expanded": "false" }, "Import");
  const gen = h("button", { type: "button", class: "btn primary", id: "m_generate" }, "Generate");
  gen.onclick = () => {
    go("seed");
    generate();
  };

  const ta = h("textarea", { rows: "2", placeholder: "Paste a settings string here", "aria-label": "Settings string to import", spellcheck: "false" });
  const imp_out = h("div", { "aria-live": "polite" });
  const load = h("button", { type: "button", class: "btn primary" }, "Load settings");
  const bar = h("div", { class: "importbar", hidden: true }, ta, h("div", { class: "row" }, load, h("button", { type: "button", class: "btn", onclick: () => toggle_import(false) }, "Close")), imp_out);
  function toggle_import(open) {
    bar.hidden = !open;
    imp_btn.setAttribute("aria-expanded", String(open));
    if (open) ta.focus();
  }
  imp_btn.onclick = () => toggle_import(bar.hidden);
  load.onclick = async () => {
    const t = ta.value.trim();
    if (!t) return fill(imp_out, notice("warn", "Paste a settings string first."));
    try {
      const r = await post("/api/settings/decode", { settings_string: t });
      state.vals = r.settings;
      touch();
      ta.value = "";
      fill(imp_out, notice("ok", "Settings loaded."), r.warnings.length ? notice("warn", r.warnings.join(" ")) : null);
      show();
    } catch (e) {
      fill(imp_out, notice("error", e.message));
    }
  };

  const box = $("#topbar");
  box.append(
    h("label", { class: "mfield", for: "m_preset" }, "Preset", preset),
    h("span", { id: "theme-slot" }),
    changed_n,
    h("div", { class: "mstring" }, str, copy_btn, imp_btn, gen),
  );
  box.after(bar);
  top = { preset, changed_n, str, copy_btn, gen };
}

const refresh_str = debounce(async () => {
  await refresh_string();
  top.str.value = live.text || live.error;
  top.copy_btn.disabled = !live.text;
}, 200);

function draw_top() {
  const schema = state.schema;
  const now = match_preset(schema, state.vals);
  fill(
    top.preset,
    h("option", { value: "" }, "Custom"),
    schema.presets.map((p) => {
      const ok = p.supported !== false;
      return h("option", { value: p.id, disabled: !ok, title: ok ? p.tip : `Not available yet in this build: ${(p.unsupported || []).join(", ")}` }, p.name);
    }),
  );
  top.preset.value = now || "";
  const n = changed(schema, state.vals).length;
  top.changed_n.textContent = n ? `${n} changed` : "All defaults";
  const reset = $("#reset-all");
  if (reset) reset.disabled = n === 0;
  const miss = gen_missing();
  top.gen.disabled = Boolean(miss) || busy();
  top.gen.title = miss ? `Fill in ${miss} on the Configuration page first.` : busy() ? "Generating" : "Generate a seed and write the mod to the mod folder";
  refresh_str();
  gen_draw();
}

export function start_menu() {
  pages = state.schema.menu || [];
  index = build_index(state.schema, pages, FIELDS);
  set_pages(pages);
  topbar();
  sidebar();
  register("config", screen(() => config_page("config", "Configuration", "Where you play, where the mod goes, and the seed")));
  register("cosmetics", screen(() => config_page("cosmetics", "Cosmetics", "Changes the mod you build, never the seed")));
  register("seed", screen(() => [head("Seed", state.result ? `Seed ${state.result.seed}` : ""), ...render_seed(show)]));
  register("plando", screen(plando_page));
  for (const p of pages) register(p.id, screen(() => settings_page(p)));
  set_first("config");
  on_show(() => {
    draw_nav();
    draw_top();
  });
  on_change(() => draw_top());
  on_run(() => {
    draw_top();
    draw_nav();
    if (current_page() === "seed" && !query.trim()) show();
  });
}
