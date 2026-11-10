export function clone(v) {
  return v === undefined ? v : JSON.parse(JSON.stringify(v));
}

export function same(a, b) {
  return JSON.stringify(a) === JSON.stringify(b);
}

export function defaults(schema) {
  const out = {};
  for (const s of schema.settings) out[s.id] = clone(s.default);
  return out;
}

export function holds(d, vals) {
  const x = vals[d.on];
  if ("is" in d) return same(x, d.is);
  if ("not" in d) return !same(x, d.not);
  if ("in" in d) return d.in.some((y) => same(x, y));
  return true;
}

export function enabled(s, vals) {
  return s.deps.every((d) => holds(d, vals));
}

export function off_value(s) {
  return "disabled" in s ? s.disabled : s.default;
}

export function max_of(s, vals) {
  let m = s.max;
  if (s.max_by && typeof vals[s.max_by] === "number") m = Math.min(m, vals[s.max_by]);
  return m;
}

export function clamp(s, vals, n) {
  const x = Number.isFinite(n) ? Math.trunc(n) : s.default;
  return Math.max(s.min, Math.min(max_of(s, vals), x));
}

function canon(s, v) {
  if (!Array.isArray(v)) return [];
  if (s.kind === "multi") {
    const order = s.choices.map((c) => c.id);
    return [...new Set(v)].filter((x) => order.includes(x)).sort((a, b) => order.indexOf(a) - order.indexOf(b));
  }
  return [...new Set(v)].sort();
}

export function effective(schema, vals) {
  const out = {};
  for (const s of schema.settings) {
    let v = s.id in vals ? clone(vals[s.id]) : clone(s.default);
    out[s.id] = v;
    if (!enabled(s, out)) {
      out[s.id] = clone(off_value(s));
      continue;
    }
    if (s.kind === "number") out[s.id] = clamp(s, out, v);
    else if (s.kind === "multi" || s.kind === "list") out[s.id] = canon(s, v);
  }
  return out;
}

export function with_value(schema, vals, id, v) {
  const out = { ...vals, [id]: v };
  for (const s of schema.settings) {
    if (s.kind === "number" && s.max_by === id) out[s.id] = clamp(s, out, out[s.id]);
  }
  return out;
}

export function from_preset(schema, p) {
  let v = defaults(schema);
  for (const [k, x] of Object.entries(p.settings)) v = with_value(schema, v, k, clone(x));
  return effective(schema, v);
}

export function match_preset(schema, vals) {
  const now = effective(schema, vals);
  for (const p of schema.presets) {
    if (same(from_preset(schema, p), now)) return p.id;
  }
  return null;
}

export function changed(schema, vals) {
  const now = effective(schema, vals);
  const base = effective(schema, defaults(schema));
  return schema.settings.filter((s) => !same(now[s.id], base[s.id]));
}

export function sanitize(schema, raw) {
  const out = defaults(schema);
  if (!raw || typeof raw !== "object") return out;
  for (const s of schema.settings) {
    if (!(s.id in raw)) continue;
    const v = raw[s.id];
    if (s.kind === "toggle" && typeof v === "boolean") out[s.id] = v;
    else if (s.kind === "choice" && s.choices.some((c) => c.id === v)) out[s.id] = v;
    else if (s.kind === "number" && Number.isInteger(v)) out[s.id] = Math.max(s.min, Math.min(s.max, v));
    else if (s.kind === "multi" && Array.isArray(v)) out[s.id] = canon(s, v);
    else if (s.kind === "list" && Array.isArray(v) && v.every((x) => typeof x === "string")) out[s.id] = v.slice(0, s.max_items);
  }
  return out;
}

export function value_label(s, v) {
  if (s.kind === "toggle") return v ? "On" : "Off";
  if (s.kind === "choice") return (s.choices.find((c) => c.id === v) || { label: String(v) }).label;
  if (s.kind === "number") return String(v);
  if (s.kind === "multi") {
    const names = v.map((x) => (s.choices.find((c) => c.id === x) || { label: x }).label);
    return names.length ? names.join(", ") : "None";
  }
  return v.length ? v.join(", ") : "None";
}

export function dep_text(schema, s) {
  const parts = [];
  for (const d of s.deps) {
    const on = schema.settings.find((x) => x.id === d.on);
    if (!on) continue;
    if ("is" in d) parts.push(`${on.label} is ${value_label(on, d.is)}`);
    else if ("not" in d) parts.push(`${on.label} is not ${value_label(on, d.not)}`);
    else if ("in" in d) parts.push(`${on.label} is ${d.in.map((y) => value_label(on, y)).join(" or ")}`);
  }
  return parts.join(" and ");
}

export function search_text(s) {
  const cs = (s.choices || []).map((c) => `${c.label} ${c.tip || ""}`).join(" ");
  return `${s.label} ${s.tip} ${s.id} ${cs}`.toLowerCase();
}
