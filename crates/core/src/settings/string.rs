use super::check::list_item_ok;
use super::codec::{Rd, b32_decode, b32_encode, put_varint, unzig, zig};
use super::model::{Kind, Schema, Setting, Value};
use super::state::Vals;
use crate::error::{Error, Res};
use crate::murmur::murmur3_32;

pub const PREFIX: &str = "OOTRR";
pub const STRING_VERSION: u32 = 1;

fn tag(body: &[u8]) -> [u8; 2] {
    let h: u32 = murmur3_32(body, 0x4F4F_5452);
    ((h ^ (h >> 16)) as u16).to_be_bytes()
}

fn wrap(mut body: Vec<u8>) -> String {
    let t: [u8; 2] = tag(&body);
    body.extend_from_slice(&t);
    format!("{PREFIX}{STRING_VERSION}-{}", b32_encode(&body))
}

fn split_prefix(t: &str) -> Res<(u32, &str)> {
    let head: &str = t.get(..PREFIX.len()).unwrap_or("");
    if !head.eq_ignore_ascii_case(PREFIX) {
        return Err(Error::Format("settings string must start with OOTRR and a version"));
    }
    let rest: &str = &t[PREFIX.len()..];
    let end: usize = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    if end == 0 || !rest[end..].starts_with('-') {
        return Err(Error::Format("settings string has no version number"));
    }
    let v: u32 = rest[..end]
        .parse()
        .map_err(|_| Error::Format("settings string version is not a number"))?;
    Ok((v, &rest[end + 1..]))
}

impl Schema {
    fn enc(&self, s: &Setting, x: &Value) -> Res<Vec<u8>> {
        let mut p: Vec<u8> = Vec::new();
        match (s.kind, x) {
            (Kind::Toggle, Value::Bool(b)) => p.push(*b as u8),
            (Kind::Choice, Value::Str(c)) => {
                let i: usize = s.choice_index(c).ok_or(Error::Format("choice vanished during encoding"))?;
                put_varint(&mut p, i as u64);
            }
            (Kind::Number, Value::Int(n)) => put_varint(&mut p, zig(*n)),
            (Kind::Multi, Value::Set(xs)) => {
                put_varint(&mut p, xs.len() as u64);
                for c in xs {
                    let i: usize = s.choice_index(c).ok_or(Error::Format("choice vanished during encoding"))?;
                    put_varint(&mut p, i as u64);
                }
            }
            (Kind::List, Value::Set(xs)) => {
                put_varint(&mut p, xs.len() as u64);
                for c in xs {
                    put_varint(&mut p, c.len() as u64);
                    p.extend_from_slice(c.as_bytes());
                }
            }
            _ => return Err(Error::Format("value does not match its setting kind")),
        }
        Ok(p)
    }

    pub fn encode(&self, v: &Vals) -> Res<String> {
        let mut n: Vals = v.clone();
        self.normalize(&mut n);
        self.validate(&n)?;
        let mut order: Vec<&Setting> = self.settings.iter().collect();
        order.sort_by_key(|s| s.num);
        let mut body: Vec<u8> = Vec::new();
        for s in order {
            let x: &Value = n.get(&s.id).ok_or(Error::Format("setting vanished during encoding"))?;
            if x == &s.default || !self.enabled(s, &n) {
                continue;
            }
            let p: Vec<u8> = self.enc(s, x)?;
            put_varint(&mut body, s.num as u64);
            put_varint(&mut body, p.len() as u64);
            body.extend_from_slice(&p);
        }
        Ok(wrap(body))
    }

    fn dec(&self, s: &Setting, p: &[u8], w: &mut Vec<String>) -> Res<Option<Value>> {
        let mut r: Rd = Rd::new(p);
        let out: Option<Value> = match s.kind {
            Kind::Toggle => {
                let b: u8 = r.take(1)?[0];
                if b > 1 {
                    return Err(Error::Format("toggle byte is not 0 or 1"));
                }
                Some(Value::Bool(b == 1))
            }
            Kind::Choice => {
                let i: u64 = r.varint()?;
                match s.choices.get(i as usize).filter(|_| i < s.choices.len() as u64) {
                    Some(c) => Some(Value::Str(c.id.clone())),
                    None => {
                        w.push(format!("setting {} has an unknown choice {i}, default used", s.id));
                        None
                    }
                }
            }
            Kind::Number => {
                let n: i64 = unzig(r.varint()?);
                if n < s.min || n > s.max {
                    w.push(format!("setting {} value {n} is out of range, default used", s.id));
                    None
                } else {
                    Some(Value::Int(n))
                }
            }
            Kind::Multi => {
                let c: u64 = r.varint()?;
                if c > r.left() as u64 {
                    return Err(Error::Format("multi-select count exceeds its data"));
                }
                let mut xs: Vec<String> = Vec::new();
                for _ in 0..c {
                    let i: u64 = r.varint()?;
                    match s.choices.get(i as usize).filter(|_| i < s.choices.len() as u64) {
                        Some(c) => xs.push(c.id.clone()),
                        None => w.push(format!("setting {} has an unknown choice {i}, skipped", s.id)),
                    }
                }
                Some(Value::Set(xs))
            }
            Kind::List => {
                let c: u64 = r.varint()?;
                if c > r.left() as u64 || c > s.max_items as u64 {
                    return Err(Error::Format("list count is too large"));
                }
                let mut xs: Vec<String> = Vec::new();
                for _ in 0..c {
                    let l: u64 = r.varint()?;
                    if l > r.left() as u64 {
                        return Err(Error::Format("list entry exceeds its data"));
                    }
                    let b: &[u8] = r.take(l as usize)?;
                    let t: String = String::from_utf8(b.to_vec())
                        .map_err(|_| Error::Format("list entry is not valid text"))?;
                    if !list_item_ok(&t) {
                        return Err(Error::Format("list entry has characters that are not allowed"));
                    }
                    xs.push(t);
                }
                Some(Value::Set(xs))
            }
        };
        if !r.done() {
            return Err(Error::Format("a setting value has trailing bytes"));
        }
        Ok(out)
    }

    pub fn decode(&self, text: &str) -> Res<(Vals, Vec<String>)> {
        let (ver, rest) = split_prefix(text.trim())?;
        if ver != STRING_VERSION {
            return Err(Error::Msg(format!(
                "settings string version {ver} is not supported, this build reads version {STRING_VERSION}"
            )));
        }
        let all: Vec<u8> = b32_decode(rest)?;
        if all.len() < 2 {
            return Err(Error::Format("settings string is too short"));
        }
        let (body, ck) = all.split_at(all.len() - 2);
        if tag(body) != ck {
            return Err(Error::Format("settings string checksum mismatch, it was mistyped or damaged"));
        }
        let mut v: Vals = self.defaults();
        let mut w: Vec<String> = Vec::new();
        let mut r: Rd = Rd::new(body);
        let mut last: Option<u64> = None;
        while !r.done() {
            let num: u64 = r.varint()?;
            let len: u64 = r.varint()?;
            if num > u16::MAX as u64 || len > r.left() as u64 {
                return Err(Error::Format("settings string entry is malformed"));
            }
            let p: &[u8] = r.take(len as usize)?;
            if last.is_some_and(|l| num <= l) {
                return Err(Error::Format("settings string entries are out of order or repeated"));
            }
            last = Some(num);
            match self.by_num(num as u16) {
                None => w.push(format!("unknown setting number {num} ignored")),
                Some(s) => {
                    if let Some(x) = self.dec(s, p, &mut w)? {
                        v.insert(s.id.clone(), x);
                    }
                }
            }
        }
        self.normalize(&mut v);
        self.validate(&v)?;
        Ok((v, w))
    }
}

#[cfg(test)]
mod tests {
    use super::super::model::Group;
    use super::*;
    use crate::rng::Rng;

    fn sch() -> Schema {
        Schema {
            version: 1,
            groups: vec![Group { id: "g".into(), label: "G".into(), tip: "t".into() }],
            settings: vec![
                Setting::choice(1, "mode", "g", "Mode", "off").tip("t").opts(&[("off", "Off"), ("stones", "Stones"), ("both", "Both")]),
                Setting::number(2, "stones", "g", "Stones", 3, -5, 300).tip("t").when("mode", "stones"),
                Setting::toggle(3, "hunt", "g", "Hunt", false).tip("t"),
                Setting::number(4, "count", "g", "Count", 30, 1, 999).tip("t").when_on("hunt"),
                Setting::multi(6, "items", "g", "Items").tip("t").opts(&[("b", "B"), ("a", "A"), ("c", "C")]),
                Setting::list(200, "skip", "g", "Skip", 3).tip("t"),
            ],
        }
    }

    fn entry(body: &mut Vec<u8>, num: u64, p: &[u8]) {
        put_varint(body, num);
        put_varint(body, p.len() as u64);
        body.extend_from_slice(p);
    }

    fn vals(s: &Schema, edits: &[(&str, Value)]) -> Vals {
        let mut v: Vals = s.defaults();
        for (k, x) in edits {
            v.insert(k.to_string(), x.clone());
        }
        s.normalize(&mut v);
        v
    }

    #[test]
    fn defaults_make_a_short_string() {
        let s: Schema = sch();
        let t: String = s.encode(&s.defaults()).unwrap();
        assert!(t.starts_with("OOTRR1-"));
        assert!(t.len() < 16, "{t}");
        assert_eq!(s.decode(&t).unwrap().0, vals(&s, &[]));
    }

    #[test]
    fn round_trip_every_kind() {
        let s: Schema = sch();
        let cases: Vec<Vals> = vec![
            vals(&s, &[("mode", Value::s("both"))]),
            vals(&s, &[("mode", Value::s("stones")), ("stones", Value::Int(-5))]),
            vals(&s, &[("mode", Value::s("stones")), ("stones", Value::Int(300))]),
            vals(&s, &[("hunt", Value::Bool(true)), ("count", Value::Int(999))]),
            vals(&s, &[("items", Value::set(&["a", "c"]))]),
            vals(&s, &[("skip", Value::set(&["Deku Tree Compass Chest", "Zora's Domain"]))]),
            vals(&s, &[
                ("mode", Value::s("stones")),
                ("stones", Value::Int(1)),
                ("hunt", Value::Bool(true)),
                ("items", Value::set(&["b", "a", "c"])),
                ("skip", Value::set(&["x"])),
            ]),
        ];
        for v in cases {
            let t: String = s.encode(&v).unwrap();
            let (back, w) = s.decode(&t).unwrap();
            assert_eq!(back, v, "{t}");
            assert!(w.is_empty());
            assert_eq!(s.encode(&back).unwrap(), t);
        }
    }

    #[test]
    fn encoding_is_deterministic_and_canonical() {
        let s: Schema = sch();
        let a: Vals = vals(&s, &[("items", Value::set(&["c", "a"])), ("mode", Value::s("both"))]);
        let mut b: Vals = s.defaults();
        b.insert("mode".into(), Value::s("both"));
        b.insert("stones".into(), Value::Int(2));
        b.insert("count".into(), Value::Int(77));
        b.insert("items".into(), Value::set(&["a", "c", "a"]));
        assert_eq!(s.encode(&a).unwrap(), s.encode(&b).unwrap());
        for _ in 0..3 {
            assert_eq!(s.encode(&a).unwrap(), s.encode(&a).unwrap());
        }
    }

    #[test]
    fn decoder_is_forgiving_about_case_and_spacing() {
        let s: Schema = sch();
        let v: Vals = vals(&s, &[("mode", Value::s("both"))]);
        let t: String = s.encode(&v).unwrap();
        let spaced: String = format!("  {}  ", t.to_lowercase().replace("-", "-\n "));
        assert_eq!(s.decode(&spaced).unwrap().0, v);
    }

    #[test]
    fn unknown_settings_and_choices_warn_instead_of_failing() {
        let s: Schema = sch();
        let mut body: Vec<u8> = Vec::new();
        entry(&mut body, 1, &[2]);
        entry(&mut body, 5, &[1, 2, 3]);
        entry(&mut body, 6, &[3, 0, 9, 2]);
        entry(&mut body, 99, &[7, 7, 7]);
        let (v, w) = s.decode(&wrap(body)).unwrap();
        assert_eq!(v["mode"], Value::s("both"));
        assert_eq!(v["items"], Value::set(&["b", "c"]));
        assert_eq!(w.len(), 3, "{w:?}");
        assert!(w.iter().any(|m| m.contains("unknown setting number 5")));
        assert!(w.iter().any(|m| m.contains("unknown setting number 99")));
        assert!(w.iter().any(|m| m.contains("unknown choice 9")));
    }

    #[test]
    fn unknown_choice_index_for_a_single_choice_falls_back_to_default() {
        let s: Schema = sch();
        let mut body: Vec<u8> = Vec::new();
        entry(&mut body, 1, &[40]);
        let (v, w) = s.decode(&wrap(body)).unwrap();
        assert_eq!(v["mode"], Value::s("off"));
        assert_eq!(w.len(), 1);
    }

    #[test]
    fn out_of_range_number_falls_back_with_a_warning() {
        let s: Schema = sch();
        let mut body: Vec<u8> = Vec::new();
        entry(&mut body, 1, &[1]);
        let mut p: Vec<u8> = Vec::new();
        put_varint(&mut p, zig(5000));
        entry(&mut body, 2, &p);
        let (v, w) = s.decode(&wrap(body)).unwrap();
        assert_eq!(v["stones"], Value::Int(3));
        assert_eq!(w.len(), 1);
    }

    #[test]
    fn stale_values_of_disabled_settings_are_dropped() {
        let s: Schema = sch();
        let mut body: Vec<u8> = Vec::new();
        let mut p: Vec<u8> = Vec::new();
        put_varint(&mut p, zig(2));
        entry(&mut body, 2, &p);
        let (v, _) = s.decode(&wrap(body)).unwrap();
        assert_eq!(v["stones"], Value::Int(3));
    }

    #[test]
    fn bad_prefix_and_versions_are_rejected() {
        let s: Schema = sch();
        let good: String = s.encode(&s.defaults()).unwrap();
        let body: &str = good.split_once('-').unwrap().1;
        for t in ["", "OOTRR", "OOTRR1", "OOTRR1-", "XOTRR1-AAAA", "OOTRR-AAAA", "OOTRRx-AAAA", "OOTRR2-AAAA", "OOTRR0-AAAA", "OOTRR99999999999-AAAA"] {
            assert!(s.decode(t).is_err(), "accepted {t:?}");
        }
        let future: String = format!("OOTRR2-{body}");
        let e: String = s.decode(&future).unwrap_err().to_string();
        assert!(e.contains("version 2"), "{e}");
        assert!(s.decode(&format!("ootrr1-{body}")).is_ok());
    }

    #[test]
    fn damage_is_detected() {
        let s: Schema = sch();
        let v: Vals = vals(&s, &[("mode", Value::s("stones")), ("stones", Value::Int(2)), ("items", Value::set(&["a"]))]);
        let t: String = s.encode(&v).unwrap();
        let (head, body) = t.split_at(7);
        let alpha: &[char] = &['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H'];
        for i in 0..body.len() {
            for a in alpha {
                let mut c: Vec<char> = body.chars().collect();
                if c[i] == *a {
                    continue;
                }
                c[i] = *a;
                let m: String = format!("{head}{}", c.iter().collect::<String>());
                assert!(s.decode(&m).is_err(), "mutation at {i} to {a} was accepted");
            }
        }
        for n in 0..body.len() {
            assert!(s.decode(&format!("{head}{}", &body[..n])).is_err(), "prefix {n} accepted");
        }
    }

    #[test]
    fn malformed_bodies_are_errors_not_panics() {
        let s: Schema = sch();
        let raw: Vec<Vec<u8>> = vec![vec![1], vec![1, 5, 0], vec![1, 1], vec![255; 11]];
        for b in raw {
            assert!(s.decode(&wrap(b.clone())).is_err(), "accepted raw body {b:?}");
        }
        let entries: Vec<Vec<(u64, Vec<u8>)>> = vec![
            vec![(3, vec![7])],
            vec![(3, vec![1, 1])],
            vec![(3, vec![])],
            vec![(1, vec![1, 1])],
            vec![(6, vec![200, 1])],
            vec![(6, vec![3])],
            vec![(6, vec![1])],
            vec![(200, vec![100])],
            vec![(200, vec![1, 9, 1])],
            vec![(200, vec![1, 2, 0xFF, 0xFE])],
            vec![(200, vec![1, 1, b'"'])],
            vec![(200, vec![1, 0])],
            vec![(200, vec![1, 1, b'a', 0])],
            vec![(2, vec![0]), (1, vec![0])],
            vec![(1, vec![0]), (1, vec![0])],
            vec![(70000, vec![0])],
        ];
        for es in entries {
            let mut body: Vec<u8> = Vec::new();
            for (n, p) in &es {
                entry(&mut body, *n, p);
            }
            assert!(s.decode(&wrap(body)).is_err(), "accepted entries {es:?}");
        }
    }

    #[test]
    fn random_bodies_never_panic() {
        let s: Schema = sch();
        let mut r: Rng = Rng::from_seed(5);
        for _ in 0..3000 {
            let n: usize = r.below(24) as usize;
            let b: Vec<u8> = (0..n).map(|_| (r.next_u64() % 8) as u8).collect();
            let _ = s.decode(&wrap(b));
        }
        for _ in 0..3000 {
            let n: usize = r.below(24) as usize;
            let b: Vec<u8> = (0..n).map(|_| r.next_u64() as u8).collect();
            let _ = s.decode(&wrap(b));
        }
    }
}
