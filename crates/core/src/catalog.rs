use std::collections::{BTreeMap as BM, HashMap as HM, HashSet as HS};
use std::fs;
use std::path::Path as P;

use crate::byml::Byml;
use crate::catalog_json::{J, parse};
use crate::error::{Error, Res};
use crate::placement::{banc_files, hash_of};
use crate::romfs::Romfs;

#[derive(Debug, Clone, PartialEq)]
pub struct Ref {
    pub hash: u64,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conf {
    Item,
    Forced,
    Pool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Via {
    Drop,
    Actor,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Loc {
    pub name: String,
    pub area: String,
    pub kind: String,
    pub vanilla: String,
    pub item: String,
    pub conf: Conf,
    pub via: Via,
    pub folder: String,
    pub age: String,
    pub pool: Option<String>,
    pub refs: Vec<Ref>,
    pub ev: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GapOot {
    pub name: String,
    pub area: String,
    pub kind: String,
    pub vanilla: String,
    pub reason: String,
    pub cands: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GapRem {
    pub folder: String,
    pub kind: String,
    pub item: String,
    pub via: Via,
    pub age: String,
    pub reason: String,
    pub refs: Vec<Ref>,
    pub cands: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Catalog {
    pub locs: Vec<Loc>,
    pub gap_oot: Vec<GapOot>,
    pub gap_rem: Vec<GapRem>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Seen {
    pub file: String,
    pub hash: u64,
    pub gyaml: String,
    pub drop: Option<String>,
}

fn text(j: &J, k: &'static str) -> Res<String> {
    j.get(k)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or(Error::Format(k))
}

fn list<'a>(j: &'a J, k: &'static str) -> Res<&'a [J]> {
    j.get(k).and_then(|v| v.as_arr()).ok_or(Error::Format(k))
}

fn hash(s: &str) -> Res<u64> {
    s.parse().map_err(|_| Error::Format("catalog: bad hash"))
}

fn via(j: &J) -> Res<Via> {
    match text(j, "via")?.as_str() {
        "drop" => Ok(Via::Drop),
        "actor" => Ok(Via::Actor),
        _ => Err(Error::Format("catalog: bad via")),
    }
}

fn refs(j: &J) -> Res<Vec<Ref>> {
    let mut out: Vec<Ref> = Vec::new();
    for r in list(j, "refs")? {
        let mut files: Vec<String> = Vec::new();
        for f in list(r, "files")? {
            files.push(f.as_str().ok_or(Error::Format("catalog: bad file"))?.to_string());
        }
        out.push(Ref { hash: hash(&text(r, "hash")?)?, files });
    }
    Ok(out)
}

fn strs(j: &J, k: &'static str) -> Res<Vec<String>> {
    let Some(a) = j.get(k) else {
        return Ok(Vec::new());
    };
    let a: &[J] = a.as_arr().ok_or(Error::Format(k))?;
    a.iter()
        .map(|v| v.as_str().map(|s| s.to_string()).ok_or(Error::Format(k)))
        .collect()
}

fn conf(s: &str) -> Res<Conf> {
    match s {
        "item" => Ok(Conf::Item),
        "forced" => Ok(Conf::Forced),
        "pool" => Ok(Conf::Pool),
        _ => Err(Error::Format("catalog: bad conf")),
    }
}

impl Catalog {
    pub fn parse(s: &str) -> Res<Catalog> {
        let j: J = parse(s)?;
        if j.get("format").and_then(|v| v.as_i64()) != Some(1) {
            return Err(Error::Format("catalog: unsupported format"));
        }
        let mut locs: Vec<Loc> = Vec::new();
        for e in list(&j, "locations")? {
            locs.push(Loc {
                name: text(e, "name")?,
                area: text(e, "area")?,
                kind: text(e, "kind")?,
                vanilla: text(e, "vanilla")?,
                item: text(e, "item")?,
                conf: conf(&text(e, "conf")?)?,
                via: via(e)?,
                folder: text(e, "folder")?,
                age: text(e, "age")?,
                pool: e.get("pool").and_then(|v| v.as_str()).map(|s| s.to_string()),
                refs: refs(e)?,
                ev: strs(e, "ev")?,
            });
        }
        let mut gap_oot: Vec<GapOot> = Vec::new();
        for e in list(&j, "unmatched_oot")? {
            let mut cands: Vec<u64> = Vec::new();
            for c in strs(e, "cands")? {
                cands.push(hash(&c)?);
            }
            gap_oot.push(GapOot {
                name: text(e, "name")?,
                area: text(e, "area")?,
                kind: text(e, "kind")?,
                vanilla: text(e, "vanilla")?,
                reason: text(e, "reason")?,
                cands,
            });
        }
        let mut gap_rem: Vec<GapRem> = Vec::new();
        for e in list(&j, "unmatched_remake")? {
            gap_rem.push(GapRem {
                folder: text(e, "folder")?,
                kind: text(e, "kind")?,
                item: text(e, "item")?,
                via: via(e)?,
                age: text(e, "age")?,
                reason: text(e, "reason")?,
                refs: refs(e)?,
                cands: strs(e, "cands")?,
            });
        }
        Ok(Catalog { locs, gap_oot, gap_rem })
    }

    pub fn load(p: &P) -> Res<Catalog> {
        let s: String = fs::read_to_string(p)?;
        Catalog::parse(&s)
    }

    pub fn by_name(&self, n: &str) -> Option<&Loc> {
        self.locs.iter().find(|l| l.name == n)
    }

    pub fn by_hash(&self, h: u64) -> Vec<&Loc> {
        self.locs.iter().filter(|l| l.refs.iter().any(|r| r.hash == h)).collect()
    }

    pub fn of_kind<'a>(&'a self, k: &'a str) -> impl Iterator<Item = &'a Loc> {
        self.locs.iter().filter(move |l| l.kind == k)
    }

    pub fn pinned(&self) -> usize {
        self.locs.iter().filter(|l| l.conf != Conf::Pool).count()
    }
}

pub fn scan(rf: &mut Romfs) -> Res<Vec<Seen>> {
    let mut out: Vec<Seen> = Vec::new();
    for f in banc_files(rf)? {
        let y: Byml = rf.read_byml(&f)?;
        let Some(list) = y.root.get("Actors").and_then(|a| a.as_arr()) else {
            continue;
        };
        for a in list {
            let Some(h) = hash_of(a) else {
                continue;
            };
            out.push(Seen {
                file: f.clone(),
                hash: h,
                gyaml: a.get("Gyaml").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                drop: a
                    .get("Dynamic")
                    .and_then(|d| d.get("Drop__DropActor"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            });
        }
    }
    Ok(out)
}

fn check_refs(
    out: &mut Vec<String>,
    who: &str,
    item: &str,
    via: Via,
    rs: &[Ref],
    idx: &HM<(&str, u64), &Seen>,
) {
    if rs.is_empty() {
        out.push(format!("{who}: no refs"));
    }
    for r in rs {
        if r.files.is_empty() {
            out.push(format!("{who}: hash {} lists no files", r.hash));
        }
        for f in &r.files {
            let Some(s) = idx.get(&(f.as_str(), r.hash)) else {
                out.push(format!("{who}: hash {} not found in {f}", r.hash));
                continue;
            };
            let have: &str = match via {
                Via::Drop => s.drop.as_deref().unwrap_or(""),
                Via::Actor => s.gyaml.as_str(),
            };
            if have != item {
                out.push(format!("{who}: hash {} in {f} holds {have:?}, expected {item:?}", r.hash));
            }
        }
    }
}

pub fn verify(c: &Catalog, seen: &[Seen]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let idx: HM<(&str, u64), &Seen> = seen.iter().map(|s| ((s.file.as_str(), s.hash), s)).collect();
    let mut names: HS<&str> = HS::new();
    let mut used: HM<(&str, u64), &str> = HM::new();
    for l in &c.locs {
        if !names.insert(l.name.as_str()) {
            out.push(format!("{}: duplicate name", l.name));
        }
        check_refs(&mut out, &l.name, &l.item, l.via, &l.refs, &idx);
        for r in &l.refs {
            if let Some(prev) = used.insert((l.folder.as_str(), r.hash), l.name.as_str()) {
                out.push(format!("{}: hash {} already used by {prev}", l.name, r.hash));
            }
        }
        if (l.conf == Conf::Pool) != l.pool.is_some() {
            out.push(format!("{}: pool id does not match conf", l.name));
        }
    }
    let mut pools: BM<&str, usize> = BM::new();
    for l in &c.locs {
        if let Some(p) = &l.pool {
            *pools.entry(p.as_str()).or_insert(0) += 1;
        }
    }
    for (p, n) in &pools {
        if *n < 2 {
            out.push(format!("{p}: pool has {n} member"));
        }
    }
    for g in &c.gap_rem {
        let who: String = format!("unmatched {} {}", g.folder, g.kind);
        check_refs(&mut out, &who, &g.item, g.via, &g.refs, &idx);
        for r in &g.refs {
            if used.contains_key(&(g.folder.as_str(), r.hash)) {
                out.push(format!("{who}: hash {} is both matched and unmatched", r.hash));
            }
        }
    }
    out
}

pub fn coverage(c: &Catalog) -> String {
    let mut m: BM<(&str, &str), [usize; 4]> = BM::new();
    for l in &c.locs {
        let e: &mut [usize; 4] = m.entry((l.area.as_str(), l.kind.as_str())).or_insert([0; 4]);
        if l.conf == Conf::Pool {
            e[1] += 1;
        } else {
            e[0] += 1;
        }
    }
    for g in &c.gap_oot {
        m.entry((g.area.as_str(), g.kind.as_str())).or_insert([0; 4])[2] += 1;
    }
    for g in &c.gap_rem {
        m.entry((g.folder.as_str(), g.kind.as_str())).or_insert([0; 4])[3] += 1;
    }
    let mut s: String = String::from("area / kind: pinned, pooled, oot unmatched, remake unmatched\n");
    let mut t: [usize; 4] = [0; 4];
    for ((a, k), v) in &m {
        s.push_str(&format!("{a} / {k}: {}, {}, {}, {}\n", v[0], v[1], v[2], v[3]));
        for i in 0..4 {
            t[i] += v[i];
        }
    }
    let n: usize = t[0] + t[1] + t[2];
    s.push_str(&format!("total: {} pinned, {} pooled, {} oot unmatched, {} remake unmatched\n", t[0], t[1], t[2], t[3]));
    if n > 0 {
        s.push_str(&format!("oot matched: {:.1}% pinned, {:.1}% including pools\n", 100.0 * t[0] as f64 / n as f64, 100.0 * (t[0] + t[1]) as f64 / n as f64));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIX: &str = "{\"format\":1,\"note\":\"x\",\"locations\":[\
{\"name\":\"A Chest\",\"area\":\"Area A\",\"kind\":\"chest\",\"vanilla\":\"Map\",\"item\":\"Item_Map\",\"conf\":\"item\",\"folder\":\"FolderA\",\"via\":\"drop\",\"age\":\"both\",\"refs\":[{\"hash\":\"18446744073709551615\",\"files\":[\"Banc/FolderA/a.zs\"]}]},\
{\"name\":\"B Chest\",\"area\":\"Area A\",\"kind\":\"chest\",\"vanilla\":\"Rupee\",\"item\":\"R_Blue\",\"conf\":\"pool\",\"folder\":\"FolderA\",\"via\":\"drop\",\"age\":\"both\",\"pool\":\"p1\",\"refs\":[{\"hash\":\"2\",\"files\":[\"Banc/FolderA/a.zs\"]}]},\
{\"name\":\"C Chest\",\"area\":\"Area A\",\"kind\":\"chest\",\"vanilla\":\"Rupee\",\"item\":\"R_Blue\",\"conf\":\"pool\",\"folder\":\"FolderA\",\"via\":\"drop\",\"age\":\"both\",\"pool\":\"p1\",\"refs\":[{\"hash\":\"3\",\"files\":[\"Banc/FolderA/a.zs\"]}]},\
{\"name\":\"A Skull\",\"area\":\"Area A\",\"kind\":\"gs\",\"vanilla\":\"Token\",\"item\":\"Enemy_Gold\",\"conf\":\"forced\",\"folder\":\"FolderA\",\"via\":\"actor\",\"age\":\"child\",\"refs\":[{\"hash\":\"4\",\"files\":[\"Banc/FolderA/a.zs\",\"Banc/FolderA/b.zs\"]}]}\
],\"unmatched_oot\":[{\"name\":\"Lost\",\"area\":\"Area B\",\"kind\":\"chest\",\"vanilla\":\"Key\",\"reason\":\"none\",\"cands\":[\"5\"]}],\
\"unmatched_remake\":[{\"folder\":\"FolderA\",\"kind\":\"chest\",\"item\":\"R_Red\",\"via\":\"drop\",\"age\":\"both\",\"reason\":\"extra\",\"refs\":[{\"hash\":\"5\",\"files\":[\"Banc/FolderA/a.zs\"]}],\"cands\":[\"Lost\"]}]}";

    fn seen(f: &str, h: u64, g: &str, d: Option<&str>) -> Seen {
        Seen { file: f.to_string(), hash: h, gyaml: g.to_string(), drop: d.map(|s| s.to_string()) }
    }

    fn dump() -> Vec<Seen> {
        vec![
            seen("Banc/FolderA/a.zs", u64::MAX, "Obj_Chest", Some("Item_Map")),
            seen("Banc/FolderA/a.zs", 2, "Obj_Chest", Some("R_Blue")),
            seen("Banc/FolderA/a.zs", 3, "Obj_Chest", Some("R_Blue")),
            seen("Banc/FolderA/a.zs", 4, "Enemy_Gold", None),
            seen("Banc/FolderA/b.zs", 4, "Enemy_Gold", None),
            seen("Banc/FolderA/a.zs", 5, "Obj_Chest", Some("R_Red")),
        ]
    }

    #[test]
    fn parses_fixture() {
        let c: Catalog = Catalog::parse(FIX).unwrap();
        assert_eq!(c.locs.len(), 4);
        assert_eq!(c.locs[0].refs[0].hash, u64::MAX);
        assert_eq!(c.locs[0].conf, Conf::Item);
        assert_eq!(c.locs[3].via, Via::Actor);
        assert_eq!(c.locs[3].refs[0].files.len(), 2);
        assert_eq!(c.gap_oot[0].cands, vec![5]);
        assert_eq!(c.gap_rem[0].cands, vec!["Lost".to_string()]);
        assert_eq!(c.pinned(), 2);
    }

    #[test]
    fn evidence_codes_are_optional_and_read_when_present() {
        let c: Catalog = Catalog::parse(FIX).unwrap();
        assert!(c.locs.iter().all(|l| l.ev.is_empty()));
        let t: String = FIX.replace("\"conf\":\"item\"", "\"conf\":\"forced\",\"ev\":[\"lbl\",\"elim\"]");
        let d: Catalog = Catalog::parse(&t).unwrap();
        assert_eq!(d.locs[0].ev, vec!["lbl".to_string(), "elim".to_string()]);
        let bad: String = FIX.replace("\"conf\":\"item\"", "\"conf\":\"item\",\"ev\":[1]");
        assert!(Catalog::parse(&bad).is_err());
    }

    #[test]
    fn looks_up_by_name_hash_and_kind() {
        let c: Catalog = Catalog::parse(FIX).unwrap();
        assert!(c.by_name("B Chest").is_some());
        assert!(c.by_name("Nope").is_none());
        assert_eq!(c.by_hash(3).len(), 1);
        assert_eq!(c.by_hash(99).len(), 0);
        assert_eq!(c.of_kind("chest").count(), 3);
    }

    #[test]
    fn clean_fixture_verifies() {
        let c: Catalog = Catalog::parse(FIX).unwrap();
        assert!(verify(&c, &dump()).is_empty());
    }

    #[test]
    fn verify_reports_missing_hash_and_wrong_item() {
        let c: Catalog = Catalog::parse(FIX).unwrap();
        let mut d: Vec<Seen> = dump();
        d.retain(|s| !(s.hash == 4 && s.file.ends_with("b.zs")));
        d[1].drop = Some("Other".to_string());
        let v: Vec<String> = verify(&c, &d);
        assert_eq!(v.len(), 2);
        assert!(v.iter().any(|m| m.contains("not found")));
        assert!(v.iter().any(|m| m.contains("expected")));
    }

    #[test]
    fn verify_reports_duplicates_and_orphan_pools() {
        let mut c: Catalog = Catalog::parse(FIX).unwrap();
        c.locs[2].refs[0].hash = 2;
        c.locs[2].name = "B Chest".to_string();
        c.locs[2].pool = None;
        let v: Vec<String> = verify(&c, &dump());
        assert!(v.iter().any(|m| m.contains("duplicate name")));
        assert!(v.iter().any(|m| m.contains("already used")));
        assert!(v.iter().any(|m| m.contains("pool id")));
        assert!(v.iter().any(|m| m.contains("pool has 1 member")));
    }

    #[test]
    fn verify_flags_hash_both_matched_and_unmatched() {
        let mut c: Catalog = Catalog::parse(FIX).unwrap();
        c.gap_rem[0].refs[0].hash = 2;
        c.gap_rem[0].item = "R_Blue".to_string();
        let v: Vec<String> = verify(&c, &dump());
        assert!(v.iter().any(|m| m.contains("both matched and unmatched")));
    }

    #[test]
    fn coverage_counts_each_bucket() {
        let c: Catalog = Catalog::parse(FIX).unwrap();
        let s: String = coverage(&c);
        assert!(s.contains("Area A / chest: 1, 2, 0, 0"));
        assert!(s.contains("Area A / gs: 1, 0, 0, 0"));
        assert!(s.contains("Area B / chest: 0, 0, 1, 0"));
        assert!(s.contains("FolderA / chest: 0, 0, 0, 1"));
        assert!(s.contains("total: 2 pinned, 2 pooled, 1 oot unmatched, 1 remake unmatched"));
    }

    #[test]
    fn rejects_bad_documents() {
        assert!(Catalog::parse("{}").is_err());
        assert!(Catalog::parse("{\"format\":2,\"locations\":[],\"unmatched_oot\":[],\"unmatched_remake\":[]}").is_err());
        let bad: String = FIX.replace("\"conf\":\"item\"", "\"conf\":\"maybe\"");
        assert!(Catalog::parse(&bad).is_err());
        let bad2: String = FIX.replace("\"18446744073709551615\"", "\"18446744073709551616\"");
        assert!(Catalog::parse(&bad2).is_err());
        let bad3: String = FIX.replace("\"via\":\"drop\"", "\"via\":\"both\"");
        assert!(Catalog::parse(&bad3).is_err());
    }
}
