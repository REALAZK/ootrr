const TABLE = (() => {
  const t = new Uint32Array(256);
  for (let i = 0; i < 256; i++) {
    let c = i;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[i] = c >>> 0;
  }
  return t;
})();

export function crc32(b) {
  let c = 0xffffffff;
  for (let i = 0; i < b.length; i++) c = TABLE[(c ^ b[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function head(size) {
  const b = new Uint8Array(size);
  return [b, new DataView(b.buffer)];
}

export function zip(files) {
  const enc = new TextEncoder();
  const parts = [];
  const central = [];
  let at = 0;
  for (const [path, data] of files) {
    const name = enc.encode(path);
    const body = data || new Uint8Array(0);
    const crc = data ? crc32(body) : 0;
    const [l, lv] = head(30);
    lv.setUint32(0, 0x04034b50, true);
    lv.setUint16(4, 20, true);
    lv.setUint16(6, 0x0800, true);
    lv.setUint16(8, 0, true);
    lv.setUint16(12, 0x21, true);
    lv.setUint32(14, crc, true);
    lv.setUint32(18, body.length, true);
    lv.setUint32(22, body.length, true);
    lv.setUint16(26, name.length, true);
    parts.push(l, name, body);
    const [c, cv] = head(46);
    cv.setUint32(0, 0x02014b50, true);
    cv.setUint16(4, 20, true);
    cv.setUint16(6, 20, true);
    cv.setUint16(8, 0x0800, true);
    cv.setUint16(14, 0x21, true);
    cv.setUint32(16, crc, true);
    cv.setUint32(20, body.length, true);
    cv.setUint32(24, body.length, true);
    cv.setUint16(28, name.length, true);
    cv.setUint32(38, data ? 0 : 0x10, true);
    cv.setUint32(42, at, true);
    central.push(c, name);
    at += 30 + name.length + body.length;
  }
  const size = central.reduce((t, b) => t + b.length, 0);
  const [e, ev] = head(22);
  ev.setUint32(0, 0x06054b50, true);
  ev.setUint16(8, files.length, true);
  ev.setUint16(10, files.length, true);
  ev.setUint32(12, size, true);
  ev.setUint32(16, at, true);
  return new Blob([...parts, ...central, e], { type: "application/zip" });
}
