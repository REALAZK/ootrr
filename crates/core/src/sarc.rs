use crate::error::{Error, Res};

const MAX_FILES: usize = 1 << 20;
const HASH_MUL: u32 = 0x65;
const DEF_ALIGN: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub data: Vec<u8>,
}

struct Rd<'a> {
    b: &'a [u8],
    be: bool,
}

fn cut() -> Error {
    Error::Format("sarc: truncated or out of range")
}

impl<'a> Rd<'a> {
    fn bytes(&self, o: usize, n: usize) -> Res<&'a [u8]> {
        let e: usize = o.checked_add(n).ok_or_else(cut)?;
        self.b.get(o..e).ok_or_else(cut)
    }

    fn u16(&self, o: usize) -> Res<u16> {
        let s: &[u8] = self.bytes(o, 2)?;
        let a: [u8; 2] = [s[0], s[1]];
        Ok(if self.be { u16::from_be_bytes(a) } else { u16::from_le_bytes(a) })
    }

    fn u32(&self, o: usize) -> Res<u32> {
        let s: &[u8] = self.bytes(o, 4)?;
        let a: [u8; 4] = [s[0], s[1], s[2], s[3]];
        Ok(if self.be { u32::from_be_bytes(a) } else { u32::from_le_bytes(a) })
    }
}

pub fn parse(b: &[u8]) -> Res<Vec<Entry>> {
    if b.get(..4) != Some(b"SARC".as_slice()) {
        return Err(Error::Format("sarc: bad magic"));
    }
    let be: bool = match b.get(6..8) {
        Some(m) if m == [0xFE, 0xFF] => true,
        Some(m) if m == [0xFF, 0xFE] => false,
        _ => return Err(Error::Format("sarc: bad byte order mark")),
    };
    let r: Rd = Rd { b, be };
    let data_off: usize = r.u32(0x0C)? as usize;
    let sfat: usize = r.u16(4)? as usize;
    if r.bytes(sfat, 4)? != b"SFAT" {
        return Err(Error::Format("sarc: missing SFAT"));
    }
    let count: usize = r.u16(sfat + 6)? as usize;
    if count > MAX_FILES {
        return Err(Error::Format("sarc: too many files"));
    }
    let nodes: usize = sfat + 12;
    let sfnt: usize = nodes + 16 * count;
    if r.bytes(sfnt, 4)? != b"SFNT" {
        return Err(Error::Format("sarc: missing SFNT"));
    }
    let names: usize = sfnt + 8;
    let mut out: Vec<Entry> = Vec::with_capacity(count);
    for i in 0..count {
        let p: usize = nodes + 16 * i;
        let hash: u32 = r.u32(p)?;
        let attr: u32 = r.u32(p + 4)?;
        let s: usize = r.u32(p + 8)? as usize;
        let e: usize = r.u32(p + 12)? as usize;
        if e < s {
            return Err(cut());
        }
        let name: String = if attr >> 24 == 1 {
            let at: usize = names + (attr & 0xFF_FFFF) as usize * 4;
            let tail: &[u8] = b.get(at..).ok_or_else(cut)?;
            let z: usize = tail.iter().position(|c| *c == 0).ok_or_else(cut)?;
            String::from_utf8(tail[..z].to_vec()).map_err(|_| Error::Format("sarc: bad utf8"))?
        } else {
            format!("{hash:08X}")
        };
        let d: &[u8] = r.bytes(data_off.checked_add(s).ok_or_else(cut)?, e - s)?;
        out.push(Entry { name, data: d.to_vec() });
    }
    Ok(out)
}

pub fn name_hash(n: &str) -> u32 {
    n.bytes().fold(0u32, |h, c| h.wrapping_mul(HASH_MUL).wrapping_add(c as u32))
}

pub fn get<'a>(e: &'a [Entry], name: &str) -> Option<&'a Entry> {
    e.iter().find(|x| x.name == name)
}

pub fn set(e: &mut Vec<Entry>, name: &str, data: Vec<u8>) -> bool {
    match e.iter_mut().find(|x| x.name == name) {
        Some(x) => {
            x.data = data;
            true
        }
        None => {
            e.push(Entry { name: name.to_string(), data });
            false
        }
    }
}

pub fn align_of(d: &[u8]) -> usize {
    match d.get(..4) {
        Some(m) if m == b"BNTX" => 0x1000,
        Some(m) if m == b"FSHA" => 0x1000,
        Some(m) if m == b"BAHS" => 0x1000,
        Some(m) if m == b"NDSR" => 0x1000,
        Some(m) if m == b"BFFH" => 0x10,
        _ => DEF_ALIGN,
    }
}

fn up(n: usize, a: usize) -> usize {
    n.div_ceil(a) * a
}

fn put(out: &mut Vec<u8>, v: u32, be: bool) {
    out.extend(if be { v.to_be_bytes() } else { v.to_le_bytes() });
}

fn put16(out: &mut Vec<u8>, v: u16, be: bool) {
    out.extend(if be { v.to_be_bytes() } else { v.to_le_bytes() });
}

pub fn write(e: &[Entry]) -> Res<Vec<u8>> {
    emit(e, false)
}

pub fn write_be(e: &[Entry]) -> Res<Vec<u8>> {
    emit(e, true)
}

fn emit(e: &[Entry], be: bool) -> Res<Vec<u8>> {
    if e.len() > u16::MAX as usize {
        return Err(Error::Format("sarc: too many files to write"));
    }
    let mut v: Vec<&Entry> = e.iter().collect();
    v.sort_by_key(|x| name_hash(&x.name));
    let mut names: Vec<u8> = Vec::new();
    let mut nodes: Vec<u8> = Vec::new();
    let mut pos: usize = 0;
    let mut top: usize = DEF_ALIGN;
    for x in &v {
        top = top.max(align_of(&x.data));
    }
    for x in &v {
        let noff: usize = names.len() / 4;
        names.extend_from_slice(x.name.as_bytes());
        names.push(0);
        names.resize(up(names.len(), 4), 0);
        let s: usize = up(pos, align_of(&x.data));
        pos = s + x.data.len();
        put(&mut nodes, name_hash(&x.name), be);
        put(&mut nodes, 0x0100_0000 | noff as u32, be);
        put(&mut nodes, u32::try_from(s).map_err(|_| Error::Format("sarc: archive too large"))?, be);
        put(&mut nodes, u32::try_from(pos).map_err(|_| Error::Format("sarc: archive too large"))?, be);
    }
    let head: usize = 0x14 + 12 + nodes.len() + 8 + names.len();
    let data_off: usize = up(head, top);
    let total: usize = data_off + pos;
    let mut out: Vec<u8> = Vec::with_capacity(total);
    out.extend_from_slice(b"SARC");
    put16(&mut out, 0x14, be);
    out.extend(if be { [0xFE, 0xFF] } else { [0xFF, 0xFE] });
    put(&mut out, u32::try_from(total).map_err(|_| Error::Format("sarc: archive too large"))?, be);
    put(&mut out, data_off as u32, be);
    put16(&mut out, 0x100, be);
    put16(&mut out, 0, be);
    out.extend_from_slice(b"SFAT");
    put16(&mut out, 12, be);
    put16(&mut out, v.len() as u16, be);
    put(&mut out, HASH_MUL, be);
    out.extend(nodes);
    out.extend_from_slice(b"SFNT");
    put16(&mut out, 8, be);
    put16(&mut out, 0, be);
    out.extend(names);
    out.resize(data_off, 0);
    for x in &v {
        let at: usize = data_off + up(out.len() - data_off, align_of(&x.data));
        out.resize(at, 0);
        out.extend_from_slice(&x.data);
    }
    Ok(out)
}

struct Slot {
    name: String,
    s: usize,
    e: usize,
    magic: [u8; 4],
}

fn slots(b: &[u8]) -> Res<(usize, Vec<Slot>)> {
    let be: bool = b.get(6..8) == Some([0xFE, 0xFF].as_slice());
    let r: Rd = Rd { b, be };
    let data_off: usize = r.u32(0x0C)? as usize;
    let es: Vec<Entry> = parse(b)?;
    let sfat: usize = r.u16(4)? as usize;
    let mut out: Vec<Slot> = Vec::with_capacity(es.len());
    for (i, x) in es.into_iter().enumerate() {
        let p: usize = sfat + 12 + 16 * i;
        let s: usize = r.u32(p + 8)? as usize;
        let e: usize = r.u32(p + 12)? as usize;
        let mut magic: [u8; 4] = [0; 4];
        for (j, c) in x.data.iter().take(4).enumerate() {
            magic[j] = *c;
        }
        out.push(Slot { name: x.name, s, e, magic });
    }
    Ok((data_off, out))
}

pub fn explain(a: &[u8], b: &[u8]) -> String {
    let (da, sa) = match slots(a) {
        Ok(v) => v,
        Err(e) => return format!("original unreadable: {e}"),
    };
    let (db, sb) = match slots(b) {
        Ok(v) => v,
        Err(e) => return format!("rewrite unreadable: {e}"),
    };
    if sa.len() != sb.len() {
        return format!("count {} vs {}", sa.len(), sb.len());
    }
    for (x, y) in sa.iter().zip(sb.iter()) {
        if x.name != y.name {
            return format!("order at {} vs {}", x.name, y.name);
        }
    }
    if da != db {
        return format!("data_off {da:#x} vs {db:#x}");
    }
    for (x, y) in sa.iter().zip(sb.iter()) {
        if x.s != y.s || x.e != y.e {
            let m: String = x.magic.iter().map(|c| if c.is_ascii_graphic() { *c as char } else { '.' }).collect();
            return format!("file {} magic {m} at {:#x}..{:#x} vs {:#x}..{:#x}", x.name, x.s, x.e, y.s, y.e);
        }
    }
    let n: usize = a.iter().zip(b.iter()).take_while(|p| p.0 == p.1).count();
    format!("bytes differ at {n:#x}, lengths {} vs {}", a.len(), b.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name_hash(n: &str) -> u32 {
        n.bytes().fold(0u32, |h, c| h.wrapping_mul(0x65).wrapping_add(c as u32))
    }

    fn build(files: &[(&str, &[u8])], be: bool) -> Vec<u8> {
        let mut fs: Vec<(&str, &[u8])> = files.to_vec();
        fs.sort_by_key(|f| name_hash(f.0));
        let p16 = |v: u16| if be { v.to_be_bytes().to_vec() } else { v.to_le_bytes().to_vec() };
        let p32 = |v: u32| if be { v.to_be_bytes().to_vec() } else { v.to_le_bytes().to_vec() };
        let mut names: Vec<u8> = Vec::new();
        let mut nodes: Vec<u8> = Vec::new();
        let mut data: Vec<u8> = Vec::new();
        for f in &fs {
            let noff: u32 = names.len() as u32 / 4;
            names.extend_from_slice(f.0.as_bytes());
            names.push(0);
            while !names.len().is_multiple_of(4) {
                names.push(0);
            }
            while !data.len().is_multiple_of(4) {
                data.push(0);
            }
            let s: u32 = data.len() as u32;
            data.extend_from_slice(f.1);
            nodes.extend(p32(name_hash(f.0)));
            nodes.extend(p32(0x0100_0000 | noff));
            nodes.extend(p32(s));
            nodes.extend(p32(data.len() as u32));
        }
        let head: usize = 0x14 + 12 + nodes.len() + 8 + names.len();
        let data_off: usize = (head + 3) & !3;
        let mut out: Vec<u8> = b"SARC".to_vec();
        out.extend(p16(0x14));
        out.extend(if be { vec![0xFE, 0xFF] } else { vec![0xFF, 0xFE] });
        out.extend(p32((data_off + data.len()) as u32));
        out.extend(p32(data_off as u32));
        out.extend(p16(0x100));
        out.extend(p16(0));
        out.extend_from_slice(b"SFAT");
        out.extend(p16(12));
        out.extend(p16(fs.len() as u16));
        out.extend(p32(0x65));
        out.extend(nodes);
        out.extend_from_slice(b"SFNT");
        out.extend(p16(8));
        out.extend(p16(0));
        out.extend(names);
        while out.len() < data_off {
            out.push(0);
        }
        out.extend(data);
        out
    }

    #[test]
    fn reads_both_byte_orders() {
        for be in [false, true] {
            let b: Vec<u8> = build(&[("a.bin", b"hello"), ("dir/b.bin", &[1, 2, 3, 4, 5, 6])], be);
            let mut e: Vec<Entry> = parse(&b).unwrap();
            e.sort_by(|x, y| x.name.cmp(&y.name));
            assert_eq!(e.len(), 2);
            assert_eq!(e[0], Entry { name: "a.bin".into(), data: b"hello".to_vec() });
            assert_eq!(e[1].name, "dir/b.bin");
            assert_eq!(e[1].data, vec![1, 2, 3, 4, 5, 6]);
        }
    }

    #[test]
    fn rejects_garbage_and_truncation() {
        assert!(parse(b"nope").is_err());
        let b: Vec<u8> = build(&[("a", b"xyz")], false);
        assert!(parse(&b[..b.len() - 2]).is_err());
        assert!(parse(&b[..20]).is_err());
    }

    fn ent(n: &str, d: &[u8]) -> Entry {
        Entry { name: n.to_string(), data: d.to_vec() }
    }

    #[test]
    fn writer_round_trips_both_byte_orders() {
        let es: Vec<Entry> = vec![ent("b/two.bin", &[1, 2, 3]), ent("one.bin", b"hello world"), ent("z", &[])];
        for be in [false, true] {
            let b: Vec<u8> = if be { write_be(&es).unwrap() } else { write(&es).unwrap() };
            let mut back: Vec<Entry> = parse(&b).unwrap();
            back.sort_by(|x, y| x.name.cmp(&y.name));
            let mut want: Vec<Entry> = es.clone();
            want.sort_by(|x, y| x.name.cmp(&y.name));
            assert_eq!(back, want);
            assert_eq!(write_or_be(&parse(&b).unwrap(), be), b);
        }
    }

    fn write_or_be(e: &[Entry], be: bool) -> Vec<u8> {
        if be { write_be(e).unwrap() } else { write(e).unwrap() }
    }

    #[test]
    fn writer_orders_nodes_by_hash_and_aligns_data() {
        let es: Vec<Entry> = vec![ent("c", &[9; 5]), ent("a", &[8; 3]), ent("b", &[7; 9])];
        let b: Vec<u8> = write(&es).unwrap();
        let got: Vec<Entry> = parse(&b).unwrap();
        let hs: Vec<u32> = got.iter().map(|x| name_hash(&x.name)).collect();
        let mut sorted: Vec<u32> = hs.clone();
        sorted.sort();
        assert_eq!(hs, sorted);
        let (d, sl) = slots(&b).unwrap();
        assert_eq!(d % 8, 0);
        for x in &sl {
            assert_eq!(x.s % 8, 0);
        }
        assert_eq!(u32::from_le_bytes([b[8], b[9], b[10], b[11]]) as usize, b.len());
    }

    #[test]
    fn writer_aligns_by_magic() {
        let mut big: Vec<u8> = b"BNTX".to_vec();
        big.extend([1u8; 20]);
        let es: Vec<Entry> = vec![ent("a", &[1; 3]), ent("t.bntx", &big), ent("f.bffh", b"BFFHxxxx")];
        let b: Vec<u8> = write(&es).unwrap();
        let (d, sl) = slots(&b).unwrap();
        assert_eq!(d % 0x1000, 0);
        for x in &sl {
            match &x.magic {
                b"BNTX" => assert_eq!(x.s % 0x1000, 0),
                b"BFFH" => assert_eq!(x.s % 0x10, 0),
                _ => assert_eq!(x.s % 8, 0),
            }
        }
    }

    #[test]
    fn get_and_set_replace_or_add() {
        let mut es: Vec<Entry> = vec![ent("a", b"1")];
        assert!(set(&mut es, "a", b"22".to_vec()));
        assert_eq!(get(&es, "a").unwrap().data, b"22".to_vec());
        assert!(!set(&mut es, "b", b"3".to_vec()));
        assert_eq!(es.len(), 2);
        assert!(get(&es, "c").is_none());
    }

    #[test]
    fn explain_reports_first_difference() {
        let a: Vec<u8> = write(&[ent("a", b"xx")]).unwrap();
        let b: Vec<u8> = write(&[ent("a", b"xx"), ent("b", b"y")]).unwrap();
        assert!(explain(&a, &b).starts_with("count"));
        let mut c: Vec<u8> = a.clone();
        let n: usize = c.len();
        c[n - 1] ^= 1;
        assert!(explain(&a, &c).starts_with("bytes differ"));
    }

    #[test]
    fn writer_rejects_too_many_files() {
        let es: Vec<Entry> = (0..70_000).map(|i| ent(&i.to_string(), &[])).collect();
        assert!(write(&es).is_err());
    }
}
