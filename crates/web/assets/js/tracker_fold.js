export const SIDE_KEY = "ootrr.tracker.side";
export const PHONE = 760;

export function fold_default(width) {
  return !(Number.isFinite(width) && width <= PHONE);
}

export function fold_parse(raw, width) {
  if (raw === "open") return true;
  if (raw === "closed") return false;
  return fold_default(width);
}

export function fold_text(open) {
  return open ? "open" : "closed";
}

export function clean_shut(v, ids) {
  const out = {};
  if (!v || typeof v !== "object") return out;
  for (const id of ids) {
    if (v[id] === true) out[id] = true;
  }
  return out;
}

export function clean_marks(v, limit) {
  const out = {};
  if (!v || typeof v !== "object") return out;
  for (const [k, n] of Object.entries(v).slice(0, limit)) {
    if (Number.isInteger(n) && n > 0 && n < 10) out[k.slice(0, 40)] = n;
  }
  return out;
}

export function next_level(now, max) {
  const n = Number.isInteger(now) && now > 0 ? now : 0;
  return (n + 1) % (max + 1);
}

export function owned(cells, state_of) {
  let n = 0;
  for (const c of cells) {
    if (state_of(c).lv > 0) n += 1;
  }
  return n;
}
