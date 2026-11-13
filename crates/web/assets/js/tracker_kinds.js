const NS = "http://www.w3.org/2000/svg";

export function s(tag, attrs, ...kids) {
  const el = document.createElementNS(NS, tag);
  if (attrs) {
    for (const [k, v] of Object.entries(attrs)) {
      if (v === null || v === undefined || v === false) continue;
      if (k.startsWith("on") && typeof v === "function") el.addEventListener(k.slice(2), v);
      else el.setAttribute(k, String(v));
    }
  }
  for (const k of kids.flat(Infinity)) {
    if (k === null || k === undefined || k === false) continue;
    el.append(k instanceof Node ? k : document.createTextNode(String(k)));
  }
  return el;
}

export const ICONS = {
  chest: "M2 7.2C2 4.3 4.4 3 8 3s6 1.3 6 4.2V8H9.3V7H6.7v1H2zM2 9.2h4.7v1.3h2.6V9.2H14V13H2z",
  token: "M8 4.5a3.5 3.5 0 1 1 0 7 3.5 3.5 0 0 1 0-7zM6.6 7a.9.9 0 1 0 0 1.8.9.9 0 0 0 0-1.8zm2.8 0a.9.9 0 1 0 0 1.8.9.9 0 0 0 0-1.8zM3.6 3.2l1.9 1.9-.7.7L2.9 3.9zm8.800 0 .7.7-1.9 1.900-.7-.7zM1.5 7.5h2.300v1H1.5zm10.700 0h2.300v1h-2.300zM4.800 10.200l.7.7-1.900 1.900-.7-.7zm6.400 0 1.900 1.900-.7.7-1.900-1.900z",
  pot: "M5 2h6v1.600l-.8.6c2 1 3.300 2.700 3.300 4.900C13.5 12.300 11.300 14 8 14s-5.500-1.700-5.500-4.900c0-2.200 1.300-3.900 3.300-4.900L5 3.600z",
  heart: "M8 14S2 10.200 2 6.100A3.100 3.100 0 0 1 8 4.900a3.100 3.100 0 0 1 6 1.200C14 10.200 8 14 8 14z",
  rupee: "M8 1.5 12.500 5v6L8 14.500 3.500 11V5zm0 2.300L5.300 5.900v4.200L8 12.200l2.700-2.100V5.900z",
  gift: "M8 2a2.600 2.600 0 1 1 0 5.200A2.600 2.600 0 0 1 8 2zM3 13.500c0-2.900 2.200-4.800 5-4.800s5 1.900 5 4.800z",
  shop: "M5.500 5V4.500a2.500 2.500 0 0 1 5 0V5H13l.8 9H2.200L3 5zm1.200 0h2.600V4.500a1.300 1.300 0 0 0-2.600 0z",
  key: "M5.500 2a3.500 3.500 0 0 1 3.400 4.400L14 11.500V14h-2.500v-1.500H10V11H8.500L7.400 9.900A3.500 3.500 0 1 1 5.500 2zm-.8 2.200a1.300 1.300 0 1 0 0 2.600 1.300 1.300 0 0 0 0-2.600z",
  flag: "M4 2h1.300v.8H12l-1.800 2.600L12 8H5.300v6H4z",
};

const KINDS = {
  chest: ["Chest", "chest", "chest"],
  trap_chest: ["Trap chest", "chest", "chest"],
  skulltula: ["Gold Skulltula", "token", "token"],
  heart_piece: ["Heart piece", "heart", "heart"],
  heart_container: ["Heart container", "heart", "heart"],
  heart: ["Recovery heart", "heart", "heart"],
  rupee: ["Rupee", "rupee", "rupee"],
  silver_rupee: ["Silver rupee", "rupee", "rupee"],
  pickup: ["Pickup", "rupee", "rupee"],
  pot: ["Pot", "pot", "pot"],
  flying_pot: ["Flying pot", "pot", "pot"],
  crate: ["Crate", "pot", "pot"],
  beehive: ["Beehive", "pot", "pot"],
  key: ["Small key", "key", "chest"],
  scrub: ["Scrub", "shop", "shop"],
  shop: ["Shop", "shop", "shop"],
  gift: ["Gift", "gift", "gift"],
  cow: ["Cow", "gift", "gift"],
  note: ["Note", "flag", "user"],
};

export const PIN_TYPES = ["note", "chest", "skulltula", "heart_piece", "rupee", "pot", "gift", "shop", "key"];

export function kind_name(k) {
  return (KINDS[k] || [k])[0];
}

export function kind_icon(k) {
  return ICONS[(KINDS[k] || [])[1]] || ICONS.flag;
}

export function kind_class(k) {
  return `k-${(KINDS[k] || [])[2] || "user"}`;
}

export function icon(k, size = 16) {
  return s("svg", { viewBox: "0 0 16 16", width: size, height: size, "aria-hidden": "true" }, s("path", { d: kind_icon(k) }));
}
