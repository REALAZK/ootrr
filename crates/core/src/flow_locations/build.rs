use std::collections::BTreeMap as BM;
use std::fmt::Write as W;
use std::path::{Path as P, PathBuf as PB};

use crate::TITLE_ID;
use crate::error::{Error as E, Res};
use crate::flow_locations::data::{path_of, Data, Edit, Loc};
use crate::flow_locations::edit::{dict_id, edit_bytes, Req};
use crate::modpack::Pack;
use crate::romfs::Romfs;
use crate::rsizetable::{self, Rst};
use crate::zs;

pub const MARGIN: u32 = 0x400;
pub const FRAME_DICT: u32 = 1;
const LISTED: usize = 60;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    pub location: String,
    pub item: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Opts {
    pub untested: bool,
    pub songs: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub location: String,
    pub file: String,
    pub index: usize,
    pub event: String,
    pub param: String,
    pub role: String,
    pub old: String,
    pub new: String,
}

#[derive(Clone, Debug)]
pub struct StagedFile {
    pub path: String,
    pub z: Vec<u8>,
    pub old: usize,
    pub new: usize,
}

#[derive(Clone, Debug)]
pub struct Stage {
    pub files: Vec<StagedFile>,
    pub changes: Vec<Change>,
    pub same: usize,
    pub untested: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Applied {
    pub root: PB,
    pub changes: Vec<Change>,
    pub locations: usize,
    pub same: usize,
    pub files: usize,
    pub grown: usize,
    pub untested: Vec<String>,
    pub report: String,
}

fn plausible(s: &str) -> bool {
    let ok: bool = s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    let known: bool = ["Item_", "Weapon_", "Armor_", "Obj_", "TBox_"].iter().any(|p| s.starts_with(p));
    ok && known && s.len() <= 64
}

fn pick<'a>(d: &'a Data, c: &Choice, o: &Opts) -> Res<&'a Loc> {
    let l: &Loc = d
        .by_name(&c.location)
        .ok_or_else(|| E::Msg(format!("{} is not a flow location this build knows", c.location)))?;
    if !l.supported && !(o.songs && l.cat == "song") {
        return Err(E::Msg(format!("{} is not supported, {}", l.name, l.why)));
    }
    Ok(l)
}

fn files_of(e: &Edit) -> Vec<&str> {
    let mut v: Vec<&str> = vec![e.file.as_str()];
    v.extend(e.also.iter().map(|s| s.as_str()));
    v
}

pub fn plan(d: &Data, cs: &[Choice], o: &Opts) -> Res<(Vec<Change>, usize, Vec<String>)> {
    let mut out: Vec<Change> = Vec::new();
    let mut same: usize = 0;
    let mut untested: Vec<String> = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    for c in cs {
        if seen.contains(&c.location.as_str()) {
            return Err(E::Msg(format!("{} is chosen twice", c.location)));
        }
        seen.push(&c.location);
        let l: &Loc = pick(d, c, o)?;
        if !d.usable(&c.item) {
            if !(o.untested && plausible(&c.item)) {
                return Err(E::Msg(format!(
                    "{} is not a name the game's own event flows hand out, so {} refuses it. Pass the untested option to try it anyway",
                    c.item, l.name
                )));
            }
            if !untested.contains(&c.item) {
                untested.push(c.item.clone());
            }
        }
        for e in &l.edits {
            if e.item == c.item {
                same += 1;
                continue;
            }
            for f in files_of(e) {
                out.push(Change {
                    location: l.name.clone(),
                    file: f.to_string(),
                    index: e.index,
                    event: e.event.clone(),
                    param: e.param.clone(),
                    role: e.role.clone(),
                    old: e.item.clone(),
                    new: c.item.clone(),
                });
            }
        }
    }
    Ok((out, same, untested))
}

fn reqs(changes: &[Change]) -> BM<String, Vec<Req>> {
    let mut by: BM<String, Vec<Req>> = BM::new();
    for c in changes {
        by.entry(c.file.clone()).or_default().push(Req {
            index: c.index,
            event: c.event.clone(),
            param: c.param.clone(),
            old: c.old.clone(),
            new: c.new.clone(),
        });
    }
    by
}

fn describe(a: &Applied, extra: &[String]) -> String {
    let mut out: String = String::new();
    let _ = writeln!(out, "Mod folder: {}", a.root.display());
    let _ = writeln!(
        out,
        "{} flow locations changed with {} event edits in {} files, {} edits already held the chosen item. {} files needed a resource size table update.",
        a.locations,
        a.changes.len(),
        a.files,
        a.same,
        a.grown
    );
    let _ = writeln!(out, "Check these in game (flow file | event | vanilla -> new | role):");
    for c in a.changes.iter().take(LISTED) {
        let _ = writeln!(
            out,
            "  {} | {} | {} ({}) | {} -> {} | {}",
            c.location, c.file, c.event, c.index, c.old, c.new, c.role
        );
    }
    if a.changes.len() > LISTED {
        let _ = writeln!(out, "  and {} more", a.changes.len() - LISTED);
    }
    if !a.untested.is_empty() {
        let _ = writeln!(out, "Untested replacement names, no game flow hands these out: {}", a.untested.join(", "));
    }
    for n in extra {
        let _ = writeln!(out, "{n}");
    }
    out
}

pub fn stage(rf: &mut Romfs, d: &Data, cs: &[Choice], o: &Opts) -> Res<Stage> {
    let (changes, same, untested): (Vec<Change>, usize, Vec<String>) = plan(d, cs, o)?;
    let by: BM<String, Vec<Req>> = reqs(&changes);
    let mut files: Vec<StagedFile> = Vec::new();
    for (file, rs) in &by {
        let path: String = path_of(file);
        let z: Vec<u8> = rf.read(&path)?;
        let id: u32 = dict_id(&z).unwrap_or(FRAME_DICT);
        let raw: Vec<u8> = rf.zs.decompress(&z)?;
        let out: Vec<u8> = edit_bytes(&raw, rs)?;
        files.push(StagedFile { path, z: zs::compress_raw(&out, Some(id))?, old: raw.len(), new: out.len() });
    }
    Ok(Stage { files, changes, same, untested })
}

pub fn apply(romfs: &P, load: &P, mod_name: &str, cs: &[Choice], o: &Opts) -> Res<Applied> {
    let d: Data = Data::load()?;
    let mut rf: Romfs = Romfs::open(romfs)?;
    let st: Stage = stage(&mut rf, &d, cs, o)?;
    let pk: Pack = Pack::create(load, TITLE_ID, mod_name)?;
    let mut rst: Option<Rst> = None;
    let mut grown: usize = 0;
    for s in &st.files {
        pk.write_romfs(&s.path, &s.z)?;
        if s.new > s.old {
            if rst.is_none() {
                rst = Some(rsizetable::load(&mut rf)?);
            }
            if let Some(t) = rst.as_mut()
                && t.grow(&s.path, s.old as u64, s.new as u64, MARGIN)?.is_some()
            {
                grown += 1;
            }
        }
    }
    if let Some(t) = &rst {
        pk.write_romfs(rsizetable::TABLE_PATH, &t.to_zs()?)?;
    }
    let mut a: Applied = Applied {
        root: pk.root,
        changes: st.changes,
        locations: cs.len(),
        same: st.same,
        files: st.files.len(),
        grown,
        untested: st.untested,
        report: String::new(),
    };
    a.report = describe(&a, &[]);
    Ok(a)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(l: &str, i: &str) -> Choice {
        Choice { location: l.to_string(), item: i.to_string() }
    }

    fn data() -> Data {
        Data::load().unwrap()
    }

    #[test]
    fn plans_the_events_of_a_supported_location_and_its_variants() {
        let d: Data = data();
        let (c, same, u): (Vec<Change>, usize, Vec<String>) =
            plan(&d, &[ch("Volvagia", "Item_Medal_Water")], &Opts::default()).unwrap();
        assert_eq!(same, 0);
        assert!(u.is_empty());
        assert_eq!(c.len(), 2);
        assert!(c.iter().all(|x| x.old == "Item_Medal_Fire" && x.new == "Item_Medal_Water" && x.index == 22));
        assert!(c.iter().any(|x| x.file.ends_with(".110.bfevfl")));
    }

    #[test]
    fn choosing_the_vanilla_item_changes_nothing() {
        let d: Data = data();
        let (c, same, _): (Vec<Change>, usize, Vec<String>) =
            plan(&d, &[ch("Volvagia", "Item_Medal_Fire")], &Opts::default()).unwrap();
        assert!(c.is_empty());
        assert_eq!(same, 1);
    }

    #[test]
    fn unsupported_unknown_and_duplicate_locations_are_refused() {
        let d: Data = data();
        let o: Opts = Opts::default();
        assert!(plan(&d, &[ch("Song from Saria", "Item_Medal_Fire")], &o).is_err());
        assert!(plan(&d, &[ch("Not A Location", "Item_Medal_Fire")], &o).is_err());
        assert!(plan(&d, &[ch("Volvagia", "Item_Medal_Water"), ch("Volvagia", "Item_Medal_Water")], &o).is_err());
    }

    #[test]
    fn replacement_names_the_game_never_hands_out_need_the_untested_option() {
        let d: Data = data();
        assert!(plan(&d, &[ch("Volvagia", "Item_Hookshot")], &Opts::default()).is_err());
        let (c, _, u): (Vec<Change>, usize, Vec<String>) =
            plan(&d, &[ch("Volvagia", "Item_Hookshot")], &Opts { untested: true, songs: false }).unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!(u, vec!["Item_Hookshot".to_string()]);
        assert!(plan(&d, &[ch("Volvagia", "bad name")], &Opts { untested: true, songs: false }).is_err());
    }

    #[test]
    fn requests_group_by_file() {
        let d: Data = data();
        let (c, _, _): (Vec<Change>, usize, Vec<String>) = plan(
            &d,
            &[ch("Queen Gohma", "Item_Medal_Fire"), ch("King Dodongo", "Item_Medal_Water")],
            &Opts::default(),
        )
        .unwrap();
        let by: BM<String, Vec<Req>> = reqs(&c);
        assert_eq!(by.len(), 2);
        assert!(by.values().all(|v| v.len() == 1));
    }

    #[test]
    fn two_locations_in_one_flow_file_share_one_request_list() {
        let d: Data = data();
        let (c, _, _): (Vec<Change>, usize, Vec<String>) = plan(
            &d,
            &[ch("Kak 10 Gold Skulltula Reward", "Item_Wallet_Large"), ch("Kak 30 Gold Skulltula Reward", "Item_Wallet_Medium")],
            &Opts::default(),
        )
        .unwrap();
        let by: BM<String, Vec<Req>> = reqs(&c);
        assert_eq!(by.len(), 1);
        assert_eq!(by.values().next().map(|v| v.len()), Some(2));
    }

    #[test]
    fn songs_are_refused_because_of_the_learning_dispatch() {
        let d: Data = data();
        for l in d.locs.iter().filter(|l| l.cat == "song") {
            assert!(!l.supported, "{}", l.name);
            assert!(l.why.contains("Dm_SY_Ocarina"), "{}", l.name);
        }
    }
}
