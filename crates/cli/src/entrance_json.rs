use std::collections::BTreeMap as BM;

use ootrr_core::entrances::{Analysis, Anc, Cat, Ent, Row, Scan, St};

pub fn esc(s: &str) -> String {
    let mut o: String = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}

fn q(s: &str) -> String {
    format!("\"{}\"", esc(s))
}

fn hs(h: u64) -> String {
    format!("\"{h}\"")
}

fn hlist(v: &[u64]) -> String {
    let p: Vec<String> = v.iter().map(|h| hs(*h)).collect();
    format!("[{}]", p.join(","))
}

fn rlist(v: &[(&'static str, u64)]) -> String {
    let p: Vec<String> = v.iter().map(|(a, h)| format!("{{\"age\":{},\"hash\":{}}}", q(a), hs(*h))).collect();
    format!("[{}]", p.join(","))
}

fn nums(v: &[f32]) -> String {
    let p: Vec<String> = v.iter().map(|x| format!("{:.3}", x)).collect();
    format!("[{}]", p.join(","))
}

fn kv(v: &[(String, String)]) -> String {
    let p: Vec<String> = v.iter().map(|(k, x)| format!("{}:{}", q(k), q(x))).collect();
    format!("{{{}}}", p.join(","))
}

pub fn swappable(e: &Ent, r: &Row) -> bool {
    let ok: bool = r.ages.iter().all(|a| if *a == "adult" { r.st_adult == St::Ok } else { r.st_child == St::Ok });
    !r.no_dest && e.init_ev.is_empty() && e.init_flag.is_empty() && e.other.is_empty() && ok
}

fn entry(s: &Scan, r: &Row) -> String {
    let e: &Ent = &s.ents[r.i];
    let ages: Vec<String> = r.ages.iter().map(|a| q(a)).collect();
    format!(
        "{{\"hash\":{},\"file\":{},\"map\":{},\"age\":{},\"name\":{},\"cat\":{},\"src_area\":{},\"dst_area\":{},\"adult\":{},\"child\":{},\"start\":{},\"first\":{},\"next\":{},\"init_event\":{},\"init_flag\":{},\"extra\":{},\"pos\":{},\"ages\":[{}],\"start_ok\":{{\"adult\":{},\"child\":{}}},\"anchors\":{},\"refs\":{},\"returns\":{},\"weak_returns\":{},\"swappable\":{}}}",
        hs(e.hash),
        q(&e.file),
        q(&e.map),
        q(&e.age),
        q(&e.name),
        q(r.cat.name()),
        q(&r.src_area),
        q(&r.dst_area),
        q(&e.adult),
        q(&e.child),
        q(&e.start),
        q(&e.first),
        q(&e.next),
        q(&e.init_ev),
        q(&e.init_flag),
        kv(&e.other),
        nums(&e.pos),
        ages.join(","),
        q(r.st_adult.name()),
        q(r.st_child.name()),
        hlist(&r.anchors),
        e.links.len(),
        rlist(&r.rets),
        rlist(&r.weak_rets),
        swappable(e, r)
    )
}

fn loose(a: &Anc) -> String {
    let g = |k: &str| -> String { a.strs.iter().find(|x| x.0 == k).map(|x| x.1.clone()).unwrap_or_default() };
    format!(
        "{{\"hash\":{},\"file\":{},\"next_param\":{},\"enable\":{},\"reverse\":{},\"pos\":{}}}",
        hs(a.hash),
        q(&a.file),
        q(&g("NextBancParamName")),
        q(&g("EnableGameData")),
        g("IsReverse") == "true",
        nums(&a.pos)
    )
}

pub fn counts(s: &Scan, an: &Analysis) -> BM<String, usize> {
    let mut c: BM<String, usize> = BM::new();
    let mut put = |k: &str, n: usize| {
        *c.entry(k.to_string()).or_insert(0) += n;
    };
    put("entrances", s.ents.len());
    for r in &an.rows {
        put(&format!("cat_{}", r.cat.name()), 1);
        put(&format!("file_age_{}", s.ents[r.i].age), 1);
        if r.no_dest {
            put("no_destination", 1);
        }
        if swappable(&s.ents[r.i], r) {
            put("swappable", 1);
        }
        if !r.rets.is_empty() {
            put("return_strict", 1);
        } else if !r.weak_rets.is_empty() {
            put("return_weak", 1);
        } else if !r.no_dest {
            put("return_none", 1);
        }
        if !r.anchors.is_empty() {
            put("with_linked_anchor", 1);
        }
    }
    put("broken_fields", an.broken.len());
    for b in &an.broken {
        put(&format!("broken_{}", b.why), 1);
    }
    put("param_anchors", an.loose.len());
    put("preload_anchors", s.ancs.len());
    put("start_positions", s.sps.len());
    put("maps", s.params.len());
    c
}

pub fn render(s: &Scan, an: &Analysis) -> String {
    let mut o: String = String::new();
    o.push_str("{\n\"format\":1,\n");
    o.push_str("\"note\":\"Identifiers only. Entrance actors are FieldAndDungeonEntranceEventRequestActor. adult and child are GameBancParam names, start is a StartPos name inside that param. ages lists the player ages that can use the actor, taken from the file name. cat is by source and destination map. broken lists every adult or child destination that does not resolve in StartPos, why is file_age (field for an age the file never serves), map_age (source map is only reachable by the other age) or live.\",\n");
    let c: BM<String, usize> = counts(s, an);
    let p: Vec<String> = c.iter().map(|(k, v)| format!("{}:{}", q(k), v)).collect();
    o.push_str(&format!("\"counts\":{{{}}},\n", p.join(",")));
    o.push_str("\"entrances\":[\n");
    let rows: Vec<String> = an.rows.iter().map(|r| entry(s, r)).collect();
    o.push_str(&rows.join(",\n"));
    o.push_str("\n],\n\"broken\":[\n");
    let bs: Vec<String> = an
        .broken
        .iter()
        .map(|b| {
            let e: &Ent = &s.ents[b.i];
            format!(
                "{{\"hash\":{},\"map\":{},\"name\":{},\"field\":{},\"param\":{},\"start\":{},\"state\":{},\"why\":{}}}",
                hs(e.hash),
                q(&e.map),
                q(&e.name),
                q(b.field),
                q(&b.param),
                q(&b.start),
                q(b.st.name()),
                q(b.why)
            )
        })
        .collect();
    o.push_str(&bs.join(",\n"));
    o.push_str("\n],\n\"unclassified\":[\n");
    let us: Vec<String> = an
        .rows
        .iter()
        .filter(|r| r.no_dest)
        .map(|r| {
            let e: &Ent = &s.ents[r.i];
            format!(
                "{{\"hash\":{},\"map\":{},\"name\":{},\"cat\":{}}}",
                hs(e.hash),
                q(&e.map),
                q(&e.name),
                q(r.cat.name())
            )
        })
        .collect();
    o.push_str(&us.join(",\n"));
    o.push_str("\n],\n\"param_anchors\":[\n");
    let ls: Vec<String> = an.loose.iter().map(|i| loose(&s.ancs[*i])).collect();
    o.push_str(&ls.join(",\n"));
    o.push_str("\n]\n}\n");
    o
}

pub fn cat_order() -> [Cat; 5] {
    [Cat::Dungeon, Cat::Overworld, Cat::Interior, Cat::Boss, Cat::Other]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes() {
        assert_eq!(esc("a\"b\\c\n"), "a\\\"b\\\\c\\n");
        assert_eq!(esc("plain"), "plain");
    }

    #[test]
    fn empty_scan_renders() {
        let s: Scan = Scan::default();
        let an: Analysis = Analysis::default();
        let t: String = render(&s, &an);
        assert!(t.starts_with("{\n\"format\":1"));
        assert!(t.contains("\"entrances\":[\n\n]"));
    }
}
