import { h, fill, toast } from "./dom.js";
import { s, kind_icon, kind_class, kind_name, icon, ICONS } from "./tracker_kinds.js";
import { ts, emit, is_just, add_pin, remember } from "./tracker_store.js";
import { places, shown_map, place_locs, pin_on, player_map } from "./tracker_places.js";
import { art_layer, W, H } from "./tracker_art.js";

const MAX_ZOOM = 10;
const cur = { svg: null, pins: [], cols: 1, place: "", cells: [], map: null };
let open_settings = () => {};

export function set_open_settings(fn) {
  open_settings = fn;
}

export function current_place() {
  const all = places();
  let pl = all.find((p) => p.id === ts.place);
  if (!pl) {
    pl = all.find((p) => p.here) || all.find((p) => p.maps.length > 0) || all[0] || null;
    ts.place = pl ? pl.id : "";
  }
  return pl;
}

export function go_place(id) {
  if (ts.place === id) return;
  ts.place = id;
  ts.floor = "all";
  ts.sel = "";
  remember();
  emit();
}

function view_of(id) {
  if (!ts.views[id]) ts.views[id] = { z: 1, cx: W / 2, cy: H / 2 };
  return ts.views[id];
}

function clamp(v) {
  v.z = Math.min(MAX_ZOOM, Math.max(1, v.z));
  const hw = W / (2 * v.z);
  const hh = H / (2 * v.z);
  v.cx = Math.min(W - hw, Math.max(hw, v.cx));
  v.cy = Math.min(H - hh, Math.max(hh, v.cy));
}

function apply_view() {
  if (!cur.svg) return;
  const v = view_of(cur.place);
  clamp(v);
  const w = W / v.z;
  const hh = H / v.z;
  cur.svg.setAttribute("viewBox", `${(v.cx - w / 2).toFixed(2)} ${(v.cy - hh / 2).toFixed(2)} ${w.toFixed(2)} ${hh.toFixed(2)}`);
  const k = (cur.cols > 1 ? cur.cols * 0.72 : 1) / Math.pow(v.z, 0.85);
  for (const p of cur.pins) p.el.setAttribute("transform", `translate(${p.x.toFixed(1)} ${p.y.toFixed(1)}) scale(${k.toFixed(3)})`);
}

function zoom_by(f, at) {
  const v = view_of(cur.place);
  const z0 = v.z;
  v.z = Math.min(MAX_ZOOM, Math.max(1, v.z * f));
  if (at) {
    const r = z0 / v.z;
    v.cx = at.x - (at.x - v.cx) * r;
    v.cy = at.y - (at.y - v.cy) * r;
  }
  apply_view();
}

function fit() {
  ts.views[cur.place] = { z: 1, cx: W / 2, cy: H / 2 };
  apply_view();
}

function point(e) {
  const m = cur.svg.getScreenCTM();
  if (!m) return { x: 0, y: 0 };
  const p = new DOMPoint(e.clientX, e.clientY).matrixTransform(m.inverse());
  return { x: p.x, y: p.y };
}

function cells_of(map) {
  const fl = (map && map.floors) || [];
  const full = (floor, label) => [{ floor, label, x: 0, y: 0, w: W, h: H }];
  if (fl.length === 0) return full(null, "");
  if (fl.length === 1) return full(0, fl[0].label);
  if (ts.floor !== "all" && fl[ts.floor]) return full(ts.floor, fl[ts.floor].label);
  const n = fl.length;
  const cols = n <= 4 ? 2 : 3;
  const rows = Math.ceil(n / cols);
  const cw = W / cols;
  const ch = H / cols;
  const y0 = (H - rows * ch) / 2;
  const out = [];
  for (let i = 0; i < n; i += 1) {
    const f = n - 1 - i;
    out.push({ floor: f, label: fl[f].label, x: (i % cols) * cw, y: y0 + Math.floor(i / cols) * ch, w: cw, h: ch });
  }
  return out;
}

function select(id) {
  ts.sel = id;
  emit();
}

function keyed(fn) {
  return (e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      fn();
    }
  };
}

function loc_pin(l, p) {
  const st = l.st || {};
  const cls = ["pin", kind_class(l.kind)];
  if (st.s === "found") cls.push("s-done");
  if (st.s === "weak") cls.push("s-weak");
  if (st.s === "lost") cls.push("s-lost");
  if (is_just(l.id)) cls.push("s-new");
  if (ts.sel === l.id) cls.push("sel");
  const state = st.s === "found" ? "collected" : st.s === "weak" ? "maybe collected" : "open";
  const go = () => select(l.id);
  const el = s(
    "g",
    {
      class: cls.join(" "),
      tabindex: "0",
      role: "button",
      "aria-label": `${l.label}, ${kind_name(l.kind)}, ${state}`,
      onclick: (e) => {
        e.stopPropagation();
        go();
      },
      onkeydown: keyed(go),
    },
    s("title", null, l.label),
    s("circle", { class: "pulse", r: 14 }),
    s("circle", { class: "halo", r: 20 }),
    s("circle", { class: "badge", r: 13 }),
    s("path", { class: "ico", d: kind_icon(l.kind), transform: "translate(-8 -8)" }),
    s("g", { class: "tick", transform: "translate(10 -10)" }, s("circle", { r: 6 }), s("path", { d: "M-3 0l2 2 4-4" })),
  );
  return { el, x: p.u * W, y: p.v * H };
}

function user_pin(p) {
  const cls = ["pin", "userpin", "k-user"];
  if (p.done) cls.push("s-done");
  if (ts.sel === `u:${p.id}`) cls.push("sel");
  const go = () => select(`u:${p.id}`);
  const el = s(
    "g",
    {
      class: cls.join(" "),
      tabindex: "0",
      role: "button",
      "aria-label": `Your pin, ${p.text || kind_name(p.kind)}${p.done ? ", done" : ""}`,
      onclick: (e) => {
        e.stopPropagation();
        go();
      },
      onkeydown: keyed(go),
    },
    s("path", { class: "body", d: "M0 0c-9-12-13-17-13-23a13 13 0 0 1 26 0c0 6-4 11-13 23z" }),
    s("path", { class: "ico", d: p.kind === "note" ? ICONS.flag : kind_icon(p.kind), transform: "translate(-7 -31) scale(0.875)" }),
    s("text", { x: 18, y: -18 }, p.text || kind_name(p.kind)),
  );
  return { el, x: p.u * W, y: p.v * H };
}

function player_mark(map) {
  const p = ts.state && ts.state.player;
  if (!p || !p.uv || player_map() !== map.id) return null;
  const el = s(
    "g",
    { class: "you", "aria-label": "Your last save is here" },
    s("title", null, "Your last save is here"),
    s("circle", { class: "player-ring", r: 13 }),
    s("path", { class: "player", d: "M0-9 7 7 0 3-7 7z" }),
  );
  return { el, x: p.uv[0] * W, y: p.uv[1] * H };
}

function build_cell(c, pl, map, locs, many) {
  const box = s("svg", { class: "cell", x: c.x, y: c.y, width: c.w, height: c.h, viewBox: `0 0 ${W} ${H}`, overflow: "visible" });
  if (many) box.append(s("rect", { class: "cellbg", x: 6, y: 6, width: W - 12, height: H - 12, rx: 18 }));
  const art = art_layer(map, c.floor);
  if (art) box.append(art);
  if (c.label) box.append(s("text", { class: "floorname", x: 28, y: 64 }, c.label));
  const first = cur.cells[0] === c;
  const on_floor = (f) => c.floor === null || f === c.floor || (f === null && first);
  const add = (p) => {
    if (!p) return;
    cur.pins.push(p);
    box.append(p.el);
  };
  if (!many) add(player_mark(map));
  for (const l of locs) {
    const p = pin_on(l, pl, map);
    if (p && on_floor(p.floor)) add(loc_pin(l, p));
  }
  if (ts.my_pins) {
    for (const p of ts.pins) {
      const here = p.map === map.id || (ts.age === "both" && pl.maps.some((m) => m.id === p.map));
      if (here && on_floor(p.floor)) add(user_pin(p));
    }
  }
  return box;
}

function drop_at(pos, pl, map) {
  const c = cur.cells.find((x) => pos.x >= x.x && pos.x <= x.x + x.w && pos.y >= x.y && pos.y <= x.y + x.h);
  if (!c) return;
  const pin = add_pin({ place: pl.id, map: map.id, floor: c.floor, u: (pos.x - c.x) / c.w, v: (pos.y - c.y) / c.h, kind: "note", text: "" });
  if (!pin) {
    toast("No room for more pins. Delete one first.");
    return;
  }
  ts.tool = "";
  ts.my_pins = true;
  select(`u:${pin.id}`);
}

function wire(svg, pl, map) {
  let drag = null;
  let moved = false;
  svg.addEventListener(
    "wheel",
    (e) => {
      e.preventDefault();
      zoom_by(e.deltaY < 0 ? 1.2 : 1 / 1.2, point(e));
    },
    { passive: false },
  );
  svg.addEventListener("pointerdown", (e) => {
    if (e.button !== 0) return;
    drag = { x: e.clientX, y: e.clientY };
    moved = false;
  });
  svg.addEventListener("pointermove", (e) => {
    if (!drag) return;
    const dx = e.clientX - drag.x;
    const dy = e.clientY - drag.y;
    if (!moved && Math.hypot(dx, dy) < 5) return;
    if (!moved) svg.setPointerCapture(e.pointerId);
    moved = true;
    const v = view_of(cur.place);
    const r = svg.getBoundingClientRect();
    const unit = Math.max(W / v.z / Math.max(1, r.width), H / v.z / Math.max(1, r.height));
    v.cx -= dx * unit;
    v.cy -= dy * unit;
    drag = { x: e.clientX, y: e.clientY };
    apply_view();
  });
  const end = () => {
    drag = null;
  };
  svg.addEventListener("pointerup", end);
  svg.addEventListener("pointercancel", end);
  svg.addEventListener("click", (e) => {
    if (moved) {
      moved = false;
      return;
    }
    if (ts.tool === "pin") drop_at(point(e), pl, map);
    else if (ts.sel) select("");
  });
}

function seg(label, cls, items, now, pick) {
  return h(
    "div",
    { class: `float seg ${cls}`, role: "group", "aria-label": label },
    items.map(([id, text]) => h("button", { type: "button", "aria-pressed": String(now === id), onclick: () => pick(id) }, text)),
  );
}

function legend() {
  const mini = (cls, kind) =>
    s(
      "svg",
      { viewBox: "-16 -16 32 32", width: 18, height: 18, class: `pin ${cls}`, "aria-hidden": "true" },
      s("circle", { class: "badge", r: 12 }),
      kind ? s("path", { class: "ico", d: kind_icon(kind), transform: "translate(-8 -8)" }) : null,
    );
  return h(
    "div",
    { class: "float legend", "aria-label": "Legend" },
    h("span", null, mini("k-chest", "chest"), "Open"),
    h("span", null, mini("k-chest s-done", "chest"), "Collected"),
    h("span", null, mini("k-user", "note"), "Your pin"),
    h("span", null, s("svg", { viewBox: "-16 -16 32 32", width: 18, height: 18, "aria-hidden": "true" }, s("path", { class: "player", d: "M0-9 7 7 0 3-7 7z" })), "Last save"),
  );
}

function title(pl, map, locs) {
  const found = locs.filter((l) => (l.st || {}).s === "found").length;
  const kind = pl.dungeon ? "Dungeon" : "Zone";
  const count = ts.mode === "hidden" ? `${found} found here` : `${found} of ${locs.length} found here`;
  const kids = [h("h1", null, pl.name), h("p", null, `${kind}. ${count}.`)];
  if (!ts.ready) {
    kids.push(h("p", { class: "warnline" }, "No map data yet. Set your game folder and the tracker draws each place from your own files."));
    kids.push(h("button", { type: "button", class: "btn pri", onclick: () => open_settings() }, "Set folders"));
  } else if (!map || map.width === null) {
    kids.push(h("p", { class: "warnline" }, "This place has no map in the game data. Use the list on the right."));
  } else {
    kids.push(h("p", { class: "fine" }, "Schematic drawn by the tracker from object positions in your game folder. It is not a game image."));
  }
  return h("div", { class: "float title" }, kids);
}

export function render_stage(box) {
  const pl = current_place();
  cur.pins = [];
  cur.svg = null;
  if (!pl) {
    fill(
      box,
      h(
        "div",
        { class: "float title" },
        h("h1", null, "Tracker"),
        h("p", null, "Nothing to show yet. Set your save folder and your game folder to start."),
        h("button", { type: "button", class: "btn pri", onclick: () => open_settings() }, "Set folders"),
      ),
    );
    return;
  }
  const map = shown_map(pl);
  const locs = place_locs(pl);
  cur.place = pl.id;
  cur.map = map;
  const kids = [];
  if (map) {
    cur.cells = cells_of(map);
    const many = cur.cells.length > 1;
    cur.cols = many ? (cur.cells.length <= 4 ? 2 : 3) : 1;
    const svg = s(
      "svg",
      { class: `map${ts.tool === "pin" ? " dropping" : ""}`, preserveAspectRatio: "xMidYMid meet", role: "group", "aria-label": `Map of ${pl.name} with pins` },
      s("defs", null, s("pattern", { id: "trk-grid", width: 40, height: 40, patternUnits: "userSpaceOnUse" }, s("path", { class: "gridline", d: "M40 0H0V40" }))),
      s("rect", { class: "gridfill", width: W, height: H, fill: "url(#trk-grid)" }),
      cur.cells.map((c) => build_cell(c, pl, map, locs, many)),
    );
    cur.svg = svg;
    wire(svg, pl, map);
    apply_view();
    kids.push(svg);
    const fl = map.floors || [];
    if (fl.length > 1) {
      const items = [["all", "All"]];
      for (let i = fl.length - 1; i >= 0; i -= 1) items.push([i, fl[i].label]);
      kids.push(
        seg("Floor", "floors", items, ts.floor, (id) => {
          ts.floor = id;
          fit();
          emit();
        }),
      );
    }
    kids.push(
      h(
        "div",
        { class: "float zoom" },
        h("button", { type: "button", "aria-label": "Zoom in", onclick: () => zoom_by(1.4) }, "+"),
        h("button", { type: "button", "aria-label": "Zoom out", onclick: () => zoom_by(1 / 1.4) }, "−"),
        h("button", { type: "button", class: "fitbtn", "aria-label": "Fit the map", onclick: fit }, "fit"),
      ),
    );
    kids.push(
      h(
        "button",
        {
          type: "button",
          class: "float tool",
          "aria-pressed": String(ts.tool === "pin"),
          title: "Then click the map where the pin goes",
          onclick: () => {
            ts.tool = ts.tool === "pin" ? "" : "pin";
            emit();
          },
        },
        icon("note", 14),
        ts.tool === "pin" ? "Click the map" : "Add a pin",
      ),
    );
    kids.push(legend());
  }
  kids.push(title(pl, map, locs));
  kids.push(
    seg(
      "Age",
      "age",
      [
        ["child", "Child"],
        ["adult", "Adult"],
        ["both", "Both"],
      ],
      ts.age,
      (id) => {
        ts.age = id;
        remember();
        emit();
      },
    ),
  );
  fill(box, kids);
}

export function map_key(e) {
  if (!cur.svg) return false;
  if (e.key === "+" || e.key === "=") zoom_by(1.4);
  else if (e.key === "-") zoom_by(1 / 1.4);
  else if (e.key === "0") fit();
  else if (e.key.startsWith("Arrow")) {
    const v = view_of(cur.place);
    const d = 80 / v.z;
    if (e.key === "ArrowLeft") v.cx -= d;
    if (e.key === "ArrowRight") v.cx += d;
    if (e.key === "ArrowUp") v.cy -= d;
    if (e.key === "ArrowDown") v.cy += d;
    apply_view();
  } else return false;
  return true;
}
