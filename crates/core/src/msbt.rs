use crate::error::{Error, Res};

const MAGIC: &[u8; 8] = b"MsgStdBn";
const HEAD: usize = 0x20;
const PAD: u8 = 0xAB;
const MAX_ITEMS: usize = 1 << 24;
const HASH_MUL: u32 = 0x492;
const OPEN: u8 = 0x0E;
const CLOSE: u8 = 0x0F;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    pub name: String,
    pub idx: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raw {
    pub tag: [u8; 4],
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sec {
    Lbl(Vec<Vec<Label>>),
    Txt(Vec<Vec<u8>>),
    Other(Raw),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Msbt {
    pub be: bool,
    pub unk_a: [u8; 2],
    pub enc: u8,
    pub ver: u8,
    pub unk_b: [u8; 2],
    pub pad: [u8; 10],
    pub secs: Vec<Sec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tok {
    Text(String),
    Code { grp: u16, kind: u16, args: Vec<u8> },
    Close { grp: u16, kind: u16 },
}

struct Rd<'a> {
    b: &'a [u8],
    be: bool,
}

fn cut() -> Error {
    Error::Format("msbt: truncated or out of range")
}

fn bad(m: &'static str) -> Error {
    Error::Format(m)
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

fn up(n: usize, a: usize) -> usize {
    n.div_ceil(a) * a
}

fn put16(out: &mut Vec<u8>, v: u16, be: bool) {
    out.extend(if be { v.to_be_bytes() } else { v.to_le_bytes() });
}

fn put32(out: &mut Vec<u8>, v: u32, be: bool) {
    out.extend(if be { v.to_be_bytes() } else { v.to_le_bytes() });
}

fn to32(n: usize) -> Res<u32> {
    u32::try_from(n).map_err(|_| bad("msbt: size does not fit in 32 bits"))
}

fn unit_of(enc: u8) -> Res<usize> {
    match enc {
        0 => Ok(1),
        1 => Ok(2),
        2 => Ok(4),
        _ => Err(bad("msbt: unknown text encoding")),
    }
}

pub fn label_hash(name: &str) -> u32 {
    name.bytes().fold(0u32, |h, c| h.wrapping_mul(HASH_MUL).wrapping_add(c as u32))
}

pub fn label_group(name: &str, groups: usize) -> usize {
    if groups == 0 { 0 } else { label_hash(name) as usize % groups }
}

fn read_lbl(d: &[u8], be: bool) -> Res<Vec<Vec<Label>>> {
    let r: Rd = Rd { b: d, be };
    let gc: usize = r.u32(0)? as usize;
    if gc > MAX_ITEMS {
        return Err(bad("msbt: too many label groups"));
    }
    let mut exp: usize = 4 + 8 * gc;
    let mut out: Vec<Vec<Label>> = Vec::with_capacity(gc);
    for g in 0..gc {
        let c: usize = r.u32(4 + 8 * g)? as usize;
        let off: usize = r.u32(8 + 8 * g)? as usize;
        if off != exp || c > MAX_ITEMS {
            return Err(bad("msbt: label table is not in canonical order"));
        }
        let mut p: usize = off;
        let mut v: Vec<Label> = Vec::new();
        for _ in 0..c {
            let l: usize = r.bytes(p, 1)?[0] as usize;
            let nm: &[u8] = r.bytes(p + 1, l)?;
            let name: String = String::from_utf8(nm.to_vec()).map_err(|_| bad("msbt: label is not utf-8"))?;
            let idx: u32 = r.u32(p + 1 + l)?;
            p += 5 + l;
            v.push(Label { name, idx });
        }
        exp = p;
        out.push(v);
    }
    if exp != d.len() {
        return Err(bad("msbt: unexpected bytes after the label table"));
    }
    Ok(out)
}

fn write_lbl(g: &[Vec<Label>], be: bool) -> Res<Vec<u8>> {
    let mut out: Vec<u8> = Vec::new();
    put32(&mut out, to32(g.len())?, be);
    let mut off: usize = 4 + 8 * g.len();
    for v in g {
        put32(&mut out, to32(v.len())?, be);
        put32(&mut out, to32(off)?, be);
        for l in v {
            off += 5 + l.name.len();
        }
    }
    for v in g {
        for l in v {
            let n: u8 = u8::try_from(l.name.len()).map_err(|_| bad("msbt: label longer than 255 bytes"))?;
            out.push(n);
            out.extend(l.name.as_bytes());
            put32(&mut out, l.idx, be);
        }
    }
    Ok(out)
}

fn read_txt(d: &[u8], be: bool, unit: usize) -> Res<Vec<Vec<u8>>> {
    let r: Rd = Rd { b: d, be };
    let n: usize = r.u32(0)? as usize;
    if n > MAX_ITEMS {
        return Err(bad("msbt: too many strings"));
    }
    let mut offs: Vec<usize> = Vec::with_capacity(n + 1);
    for i in 0..n {
        offs.push(r.u32(4 + 4 * i)? as usize);
    }
    offs.push(d.len());
    if (n > 0 && offs[0] != 4 + 4 * n) || (n == 0 && d.len() != 4) {
        return Err(bad("msbt: string table is not in canonical order"));
    }
    let mut out: Vec<Vec<u8>> = Vec::with_capacity(n);
    for i in 0..n {
        if offs[i] > offs[i + 1] {
            return Err(bad("msbt: string offsets are not ascending"));
        }
        let s: &[u8] = r.bytes(offs[i], offs[i + 1] - offs[i])?;
        if s.len() < unit || s[s.len() - unit..].iter().any(|c| *c != 0) {
            return Err(bad("msbt: string has no terminator"));
        }
        out.push(s[..s.len() - unit].to_vec());
    }
    Ok(out)
}

fn write_txt(v: &[Vec<u8>], be: bool, unit: usize) -> Res<Vec<u8>> {
    let mut out: Vec<u8> = Vec::new();
    put32(&mut out, to32(v.len())?, be);
    let mut off: usize = 4 + 4 * v.len();
    for s in v {
        put32(&mut out, to32(off)?, be);
        off += s.len() + unit;
    }
    for s in v {
        out.extend(s);
        out.extend(std::iter::repeat_n(0u8, unit));
    }
    Ok(out)
}

pub fn parse(b: &[u8]) -> Res<Msbt> {
    if b.get(..8) != Some(MAGIC.as_slice()) {
        return Err(bad("msbt: bad magic"));
    }
    let be: bool = match b.get(8..10) {
        Some(m) if m == [0xFE, 0xFF] => true,
        Some(m) if m == [0xFF, 0xFE] => false,
        _ => return Err(bad("msbt: bad byte order mark")),
    };
    let r: Rd = Rd { b, be };
    let h: &[u8] = r.bytes(0, HEAD)?;
    let enc: u8 = h[12];
    let unit: usize = unit_of(enc)?;
    let n: usize = r.u16(14)? as usize;
    if r.u32(18)? as usize != b.len() {
        return Err(bad("msbt: file size field does not match"));
    }
    let mut pos: usize = HEAD;
    let mut secs: Vec<Sec> = Vec::with_capacity(n);
    for _ in 0..n {
        let tag: [u8; 4] = [r.bytes(pos, 4)?[0], b[pos + 1], b[pos + 2], b[pos + 3]];
        let sz: usize = r.u32(pos + 4)? as usize;
        if r.bytes(pos + 8, 8)?.iter().any(|c| *c != 0) {
            return Err(bad("msbt: section header reserved bytes are not zero"));
        }
        let d: &[u8] = r.bytes(pos + 16, sz)?;
        let end: usize = pos + 16 + sz;
        let next: usize = up(end, 16);
        if r.bytes(end, next - end)?.iter().any(|c| *c != PAD) {
            return Err(bad("msbt: section padding is not 0xAB"));
        }
        secs.push(match &tag {
            b"LBL1" => Sec::Lbl(read_lbl(d, be)?),
            b"TXT2" => Sec::Txt(read_txt(d, be, unit)?),
            _ => Sec::Other(Raw { tag, data: d.to_vec() }),
        });
        pos = next;
    }
    if pos != b.len() {
        return Err(bad("msbt: unexpected bytes after the last section"));
    }
    Ok(Msbt {
        be,
        unk_a: [h[10], h[11]],
        enc,
        ver: h[13],
        unk_b: [h[16], h[17]],
        pad: [h[22], h[23], h[24], h[25], h[26], h[27], h[28], h[29], h[30], h[31]],
        secs,
    })
}

impl Msbt {
    pub fn labels(&self) -> Option<&Vec<Vec<Label>>> {
        self.secs.iter().find_map(|s| if let Sec::Lbl(g) = s { Some(g) } else { None })
    }

    pub fn strings(&self) -> Option<&Vec<Vec<u8>>> {
        self.secs.iter().find_map(|s| if let Sec::Txt(v) = s { Some(v) } else { None })
    }

    fn strings_mut(&mut self) -> Option<&mut Vec<Vec<u8>>> {
        self.secs.iter_mut().find_map(|s| if let Sec::Txt(v) = s { Some(v) } else { None })
    }

    pub fn names(&self) -> Vec<(&str, u32)> {
        let mut out: Vec<(&str, u32)> = Vec::new();
        if let Some(g) = self.labels() {
            for v in g {
                for l in v {
                    out.push((l.name.as_str(), l.idx));
                }
            }
        }
        out.sort_by_key(|x| x.1);
        out
    }

    pub fn find(&self, name: &str) -> Option<usize> {
        let g: &Vec<Vec<Label>> = self.labels()?;
        let hit: Option<&Label> = g
            .get(label_group(name, g.len()))
            .and_then(|v| v.iter().find(|l| l.name == name))
            .or_else(|| g.iter().flatten().find(|l| l.name == name));
        hit.map(|l| l.idx as usize)
    }

    pub fn raw(&self, name: &str) -> Option<&[u8]> {
        let i: usize = self.find(name)?;
        self.strings()?.get(i).map(|v| v.as_slice())
    }

    pub fn text(&self, name: &str) -> Res<Vec<Tok>> {
        let b: &[u8] = self.raw(name).ok_or(bad("msbt: no such label"))?;
        self.decode(b)
    }

    pub fn decode(&self, b: &[u8]) -> Res<Vec<Tok>> {
        if self.enc != 0 {
            return Err(bad("msbt: text decoding supports utf-8 files only"));
        }
        decode(b, self.be)
    }

    pub fn encode(&self, t: &[Tok]) -> Res<Vec<u8>> {
        if self.enc != 0 {
            return Err(bad("msbt: text encoding supports utf-8 files only"));
        }
        encode(t, self.be)
    }

    pub fn set_raw(&mut self, name: &str, v: Vec<u8>) -> Res<()> {
        let i: usize = self.find(name).ok_or(bad("msbt: no such label"))?;
        let s: &mut Vec<Vec<u8>> = self.strings_mut().ok_or(bad("msbt: no TXT2 section"))?;
        let slot: &mut Vec<u8> = s.get_mut(i).ok_or(bad("msbt: label points past the string table"))?;
        *slot = v;
        Ok(())
    }

    pub fn set_tokens(&mut self, name: &str, t: &[Tok]) -> Res<()> {
        let v: Vec<u8> = self.encode(t)?;
        self.set_raw(name, v)
    }

    pub fn set_markup(&mut self, name: &str, s: &str) -> Res<()> {
        let t: Vec<Tok> = from_markup(s)?;
        self.set_tokens(name, &t)
    }

    pub fn add_raw(&mut self, name: &str, v: Vec<u8>) -> Res<usize> {
        if name.is_empty() || name.len() > 255 {
            return Err(bad("msbt: a new label needs 1 to 255 bytes"));
        }
        if self.find(name).is_some() {
            return Err(bad("msbt: the label exists already"));
        }
        if self.secs.iter().any(|s| matches!(s, Sec::Other(_))) {
            return Err(bad("msbt: cannot add a label to a file with sections besides LBL1 and TXT2"));
        }
        let n: usize = self.strings().ok_or(bad("msbt: no TXT2 section"))?.len();
        let idx: u32 = to32(n)?;
        let g: &mut Vec<Vec<Label>> = self
            .secs
            .iter_mut()
            .find_map(|s| if let Sec::Lbl(g) = s { Some(g) } else { None })
            .ok_or(bad("msbt: no LBL1 section"))?;
        if g.is_empty() {
            return Err(bad("msbt: the label table has no groups"));
        }
        let at: usize = label_group(name, g.len());
        g[at].push(Label { name: name.to_string(), idx });
        self.strings_mut().ok_or(bad("msbt: no TXT2 section"))?.push(v);
        Ok(n)
    }

    pub fn put_markup(&mut self, name: &str, s: &str) -> Res<bool> {
        let t: Vec<Tok> = from_markup(s)?;
        self.put_tokens(name, &t)
    }

    pub fn put_tokens(&mut self, name: &str, t: &[Tok]) -> Res<bool> {
        let v: Vec<u8> = self.encode(t)?;
        if self.find(name).is_some() {
            self.set_raw(name, v)?;
            return Ok(false);
        }
        self.add_raw(name, v)?;
        Ok(true)
    }

    pub fn write(&self) -> Res<Vec<u8>> {
        let unit: usize = unit_of(self.enc)?;
        let be: bool = self.be;
        let mut body: Vec<u8> = Vec::new();
        for s in &self.secs {
            let (tag, d): ([u8; 4], Vec<u8>) = match s {
                Sec::Lbl(g) => (*b"LBL1", write_lbl(g, be)?),
                Sec::Txt(v) => (*b"TXT2", write_txt(v, be, unit)?),
                Sec::Other(r) => (r.tag, r.data.clone()),
            };
            body.extend(tag);
            put32(&mut body, to32(d.len())?, be);
            body.extend([0u8; 8]);
            body.extend(&d);
            let n: usize = up(body.len(), 16);
            body.resize(n, PAD);
        }
        let mut out: Vec<u8> = Vec::with_capacity(HEAD + body.len());
        out.extend(MAGIC);
        out.extend(if be { [0xFE, 0xFF] } else { [0xFF, 0xFE] });
        out.extend(self.unk_a);
        out.push(self.enc);
        out.push(self.ver);
        put16(&mut out, u16::try_from(self.secs.len()).map_err(|_| bad("msbt: too many sections"))?, be);
        out.extend(self.unk_b);
        put32(&mut out, to32(HEAD + body.len())?, be);
        out.extend(self.pad);
        out.extend(body);
        Ok(out)
    }
}

pub fn replace_label(file: &[u8], name: &str, markup: &str) -> Res<Vec<u8>> {
    let mut m: Msbt = parse(file)?;
    m.set_markup(name, markup)?;
    m.write()
}

pub fn decode(b: &[u8], be: bool) -> Res<Vec<Tok>> {
    let r: Rd = Rd { b, be };
    let mut out: Vec<Tok> = Vec::new();
    let mut i: usize = 0;
    while i < b.len() {
        match b[i] {
            OPEN => {
                let grp: u16 = r.u16(i + 1)?;
                let kind: u16 = r.u16(i + 3)?;
                let n: usize = r.u16(i + 5)? as usize;
                let args: Vec<u8> = r.bytes(i + 7, n)?.to_vec();
                out.push(Tok::Code { grp, kind, args });
                i += 7 + n;
            }
            CLOSE => {
                let grp: u16 = r.u16(i + 1)?;
                let kind: u16 = r.u16(i + 3)?;
                out.push(Tok::Close { grp, kind });
                i += 5;
            }
            _ => {
                let j: usize = b[i..].iter().position(|c| *c == OPEN || *c == CLOSE).map_or(b.len(), |k| i + k);
                let s: String = String::from_utf8(b[i..j].to_vec()).map_err(|_| bad("msbt: text is not utf-8"))?;
                out.push(Tok::Text(s));
                i = j;
            }
        }
    }
    Ok(out)
}

pub fn encode(t: &[Tok], be: bool) -> Res<Vec<u8>> {
    let mut out: Vec<u8> = Vec::new();
    for k in t {
        match k {
            Tok::Text(s) => {
                if s.bytes().any(|c| c == OPEN || c == CLOSE) {
                    return Err(bad("msbt: text holds a raw control byte"));
                }
                out.extend(s.as_bytes());
            }
            Tok::Code { grp, kind, args } => {
                out.push(OPEN);
                put16(&mut out, *grp, be);
                put16(&mut out, *kind, be);
                put16(&mut out, u16::try_from(args.len()).map_err(|_| bad("msbt: control code arguments too long"))?, be);
                out.extend(args);
            }
            Tok::Close { grp, kind } => {
                out.push(CLOSE);
                put16(&mut out, *grp, be);
                put16(&mut out, *kind, be);
            }
        }
    }
    Ok(out)
}

pub fn to_markup(t: &[Tok]) -> String {
    let mut out: String = String::new();
    for k in t {
        match k {
            Tok::Text(s) => {
                for c in s.chars() {
                    match c {
                        '{' => out.push_str("{{"),
                        '}' => out.push_str("}}"),
                        _ => out.push(c),
                    }
                }
            }
            Tok::Code { grp, kind, args } => {
                out.push_str(&format!("{{{grp}.{kind}"));
                if !args.is_empty() {
                    out.push(':');
                    for a in args {
                        out.push_str(&format!("{a:02x}"));
                    }
                }
                out.push('}');
            }
            Tok::Close { grp, kind } => out.push_str(&format!("{{/{grp}.{kind}}}")),
        }
    }
    out
}

fn pair(s: &str) -> Res<(u16, u16)> {
    let (a, b): (&str, &str) = s.split_once('.').ok_or(bad("msbt: markup code needs group.kind"))?;
    let g: u16 = a.parse().map_err(|_| bad("msbt: bad markup group"))?;
    let k: u16 = b.parse().map_err(|_| bad("msbt: bad markup kind"))?;
    Ok((g, k))
}

fn unhex(s: &str) -> Res<Vec<u8>> {
    if !s.len().is_multiple_of(2) || !s.is_ascii() {
        return Err(bad("msbt: bad markup hex arguments"));
    }
    let b: &[u8] = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len() / 2);
    for i in (0..b.len()).step_by(2) {
        let h: &str = std::str::from_utf8(&b[i..i + 2]).map_err(|_| bad("msbt: bad markup hex arguments"))?;
        out.push(u8::from_str_radix(h, 16).map_err(|_| bad("msbt: bad markup hex arguments"))?);
    }
    Ok(out)
}

pub fn from_markup(s: &str) -> Res<Vec<Tok>> {
    let b: &[u8] = s.as_bytes();
    let mut out: Vec<Tok> = Vec::new();
    let mut cur: String = String::new();
    let mut i: usize = 0;
    while i < b.len() {
        let c: u8 = b[i];
        let nx: Option<&u8> = b.get(i + 1);
        if c == b'{' && nx == Some(&b'{') {
            cur.push('{');
            i += 2;
        } else if c == b'}' && nx == Some(&b'}') {
            cur.push('}');
            i += 2;
        } else if c == b'}' {
            return Err(bad("msbt: stray closing brace in markup"));
        } else if c == b'{' {
            let j: usize = s[i..].find('}').map(|k| i + k).ok_or(bad("msbt: unterminated markup code"))?;
            let inner: &str = &s[i + 1..j];
            if !cur.is_empty() {
                out.push(Tok::Text(std::mem::take(&mut cur)));
            }
            if let Some(r) = inner.strip_prefix('/') {
                let (grp, kind): (u16, u16) = pair(r)?;
                out.push(Tok::Close { grp, kind });
            } else {
                let (h, a): (&str, &str) = inner.split_once(':').unwrap_or((inner, ""));
                let (grp, kind): (u16, u16) = pair(h)?;
                out.push(Tok::Code { grp, kind, args: unhex(a)? });
            }
            i = j + 1;
        } else {
            let ch: char = s[i..].chars().next().ok_or(bad("msbt: bad markup text"))?;
            cur.push(ch);
            i += ch.len_utf8();
        }
    }
    if !cur.is_empty() {
        out.push(Tok::Text(cur));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(be: bool, extra: bool) -> Msbt {
        let mut secs: Vec<Sec> = Vec::new();
        let groups: usize = 3;
        let names: [&str; 3] = ["TEST_A", "TEST_B", "TEST_LONGER_C"];
        let mut g: Vec<Vec<Label>> = vec![Vec::new(); groups];
        for (i, n) in names.iter().enumerate() {
            g[label_group(n, groups)].push(Label { name: n.to_string(), idx: i as u32 });
        }
        secs.push(Sec::Lbl(g));
        let t: Vec<Vec<u8>> = vec![
            b"plain text".to_vec(),
            encode(&[Tok::Text("a".into()), Tok::Code { grp: 3, kind: 0, args: vec![1, 2, 3, 4] }, Tok::Text("b".into())], be).unwrap(),
            Vec::new(),
        ];
        secs.push(Sec::Txt(t));
        if extra {
            secs.push(Sec::Other(Raw { tag: *b"ATR1", data: vec![9, 8, 7, 6, 5] }));
        }
        Msbt { be, unk_a: [0, 0], enc: 0, ver: 3, unk_b: [0, 0], pad: [0; 10], secs }
    }

    #[test]
    fn round_trip_both_orders() {
        for be in [false, true] {
            for extra in [false, true] {
                let m: Msbt = sample(be, extra);
                let b: Vec<u8> = m.write().unwrap();
                assert!(!b.is_empty());
                assert_eq!(b.len() % 16, 0);
                let p: Msbt = parse(&b).unwrap();
                assert_eq!(p, m);
                assert_eq!(p.write().unwrap(), b);
            }
        }
    }

    #[test]
    fn lookup_and_edit() {
        let m: Msbt = sample(false, true);
        let b: Vec<u8> = m.write().unwrap();
        let out: Vec<u8> = replace_label(&b, "TEST_B", "new {3.0:0a0b} text {{ok}}").unwrap();
        let p: Msbt = parse(&out).unwrap();
        assert_eq!(p.find("TEST_A"), Some(0));
        assert_eq!(p.find("TEST_LONGER_C"), Some(2));
        assert_eq!(p.find("NOPE"), None);
        assert_eq!(p.raw("TEST_A"), Some(b"plain text".as_slice()));
        let t: Vec<Tok> = p.text("TEST_B").unwrap();
        assert_eq!(
            t,
            vec![
                Tok::Text("new ".into()),
                Tok::Code { grp: 3, kind: 0, args: vec![10, 11] },
                Tok::Text(" text {ok}".into())
            ]
        );
        assert_eq!(p.secs.last(), m.secs.last());
        assert!(replace_label(&b, "NOPE", "x").is_err());
    }

    #[test]
    fn a_new_label_lands_in_its_hash_group_and_reads_back() {
        let mut m: Msbt = sample(false, false);
        assert!(m.put_markup("OOTRR_NEW", "one {0.3:0000}two").unwrap());
        assert_eq!(m.find("OOTRR_NEW"), Some(3));
        let g: &Vec<Vec<Label>> = m.labels().unwrap();
        assert!(g[label_group("OOTRR_NEW", 3)].iter().any(|l| l.name == "OOTRR_NEW" && l.idx == 3));
        let b: Vec<u8> = m.write().unwrap();
        let p: Msbt = parse(&b).unwrap();
        assert_eq!(p, m);
        assert_eq!(p.write().unwrap(), b);
        assert_eq!(p.raw("TEST_A"), Some(b"plain text".as_slice()));
        assert_eq!(to_markup(&p.text("OOTRR_NEW").unwrap()), "one {0.3:0000}two");
        let mut q: Msbt = p.clone();
        assert!(!q.put_markup("OOTRR_NEW", "three").unwrap());
        assert_eq!(q.strings().unwrap().len(), 4);
        assert_eq!(q.raw("OOTRR_NEW"), Some(b"three".as_slice()));
        assert!(q.add_raw("OOTRR_NEW", Vec::new()).is_err());
        assert!(q.add_raw("", Vec::new()).is_err());
        let mut odd: Msbt = sample(false, true);
        assert!(odd.add_raw("OOTRR_NEW", Vec::new()).is_err());
    }

    #[test]
    fn markup_round_trip() {
        let t: Vec<Tok> = vec![
            Tok::Text("a {b} c".into()),
            Tok::Code { grp: 5, kind: 1, args: vec![] },
            Tok::Close { grp: 5, kind: 1 },
            Tok::Code { grp: 201, kind: 13, args: vec![0xff, 0x00] },
            Tok::Text("\u{e9}\n".into()),
        ];
        let m: String = to_markup(&t);
        assert_eq!(from_markup(&m).unwrap(), t);
        for be in [false, true] {
            let e: Vec<u8> = encode(&t, be).unwrap();
            assert_eq!(decode(&e, be).unwrap(), t);
        }
        assert!(from_markup("{").is_err());
        assert!(from_markup("}").is_err());
        assert!(from_markup("{3}").is_err());
        assert!(from_markup("{3.0:abc}").is_err());
    }

    #[test]
    fn rejects_damage() {
        let b: Vec<u8> = sample(false, false).write().unwrap();
        assert!(parse(&b[..b.len() - 1]).is_err());
        assert!(parse(&[]).is_err());
        let mut c: Vec<u8> = b.clone();
        c[0] = b'X';
        assert!(parse(&c).is_err());
        let mut d: Vec<u8> = b.clone();
        let n: usize = d.len();
        d[n - 1] = 0;
        assert!(parse(&d).is_err());
    }

    #[test]
    fn label_hash_matches_known_layout() {
        assert_eq!(label_hash(""), 0);
        assert_eq!(label_hash("a"), 97);
        assert_eq!(label_hash("ab"), 97 * 0x492 + 98);
        assert_eq!(label_group("x", 0), 0);
    }
}
