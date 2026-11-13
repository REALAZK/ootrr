pub mod anju;
pub mod poe;
pub mod skip;

use std::fs;
use std::path::Path as P;

use crate::error::{Error, Res};
use crate::flow_locations::dict_id;
use crate::romfs::Romfs;
use crate::settings::Settings;
use crate::staging::{self, Out};
use crate::zs;

pub const SKIP: &str = "skip_cutscenes";
pub const ANJU: &str = "anju_one_cucco";
pub const POE: &str = "big_poe_one";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Skip {
    #[default]
    Off,
    Story,
    All,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Opts {
    pub skip: Skip,
    pub anju: bool,
    pub poe: bool,
}

pub type Have<'a> = &'a dyn Fn(&str) -> Option<Vec<u8>>;

impl Opts {
    pub fn any(&self) -> bool {
        self.skip != Skip::Off || self.anju || self.poe
    }

    pub fn logic(&self) -> Vec<(&'static str, &'static str)> {
        let mut out: Vec<(&'static str, &'static str)> = Vec::new();
        if self.anju {
            out.push((anju::LOGIC_KEY, "1"));
        }
        if self.poe {
            out.push((poe::LOGIC_KEY, "1"));
        }
        out
    }

    pub fn notes(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        match self.skip {
            Skip::Off => {}
            Skip::Story => out.push(skip::NOTE_STORY.to_string()),
            Skip::All => out.push(skip::NOTE_ALL.to_string()),
        }
        if self.anju {
            out.push(anju::NOTE.to_string());
        }
        if self.poe {
            out.push(poe::NOTE.to_string());
        }
        out
    }
}

pub fn read(s: &Settings, bad: &mut Vec<String>) -> Opts {
    let skip: Skip = match s.text(SKIP) {
        Ok("off") => Skip::Off,
        Ok("story") => Skip::Story,
        Ok("all") => Skip::All,
        Ok(v) => {
            bad.push(format!("{SKIP} is {v}, which is not a known choice"));
            Skip::Off
        }
        Err(e) => {
            bad.push(e.to_string());
            Skip::Off
        }
    };
    let mut flag = |id: &str| -> bool {
        s.flag(id).unwrap_or_else(|e| {
            bad.push(e.to_string());
            false
        })
    };
    let anju: bool = flag(ANJU);
    let poe: bool = flag(POE);
    Opts { skip, anju, poe }
}

pub fn files() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    out.push(skip::FILE.to_string());
    out.extend(skip::LISTS.iter().map(|s| s.to_string()));
    out.push(anju::FILE.to_string());
    out.push(poe::FILE.to_string());
    out
}

fn source(rf: &Romfs, have: Have, path: &str) -> Res<Vec<u8>> {
    match have(path) {
        Some(b) => Ok(b),
        None => rf.read(path),
    }
}

pub fn edit_plain(rf: &Romfs, have: Have, path: &str, f: impl FnOnce(&[u8]) -> Res<Vec<u8>>) -> Res<Out> {
    let raw: Vec<u8> = source(rf, have, path)?;
    let out: Vec<u8> = f(&raw)?;
    Ok(Out { path: path.to_string(), old: raw.len(), new: out.len(), z: out })
}

pub fn edit_zs(rf: &mut Romfs, have: Have, path: &str, f: impl FnOnce(&[u8]) -> Res<Vec<u8>>) -> Res<Out> {
    let z: Vec<u8> = source(rf, have, path)?;
    let id: Option<u32> = dict_id(&z);
    let raw: Vec<u8> = rf.zs.decompress(&z)?;
    let out: Vec<u8> = f(&raw)?;
    Ok(Out { path: path.to_string(), old: raw.len(), new: out.len(), z: zs::compress_raw(&out, id)? })
}

pub fn stage(rf: &mut Romfs, o: &Opts, have: Have) -> Res<Vec<Out>> {
    let mut outs: Vec<Out> = Vec::new();
    outs.extend(skip::stage(rf, o.skip, have)?);
    if o.anju {
        outs.push(anju::stage(rf, have)?);
    }
    if o.poe {
        outs.push(poe::stage(rf, have)?);
    }
    Ok(outs)
}

pub fn build(romfs: &P, load: &P, name: &str, o: &Opts) -> Res<String> {
    if !o.any() {
        return Err(Error::Format("qol: nothing selected"));
    }
    let mut rf: Romfs = Romfs::open(romfs)?;
    let outs: Vec<Out> = stage(&mut rf, o, &|_: &str| None)?;
    let c: staging::Committed = staging::commit(&mut rf, load, name, &outs, None)?;
    Ok(c.report)
}

pub fn worked(rf: &mut Romfs, pk: &crate::modpack::Pack, o: &Opts) -> Res<Vec<Out>> {
    let root: std::path::PathBuf = pk.work.join("romfs");
    let have = |p: &str| -> Option<Vec<u8>> { fs::read(p.split('/').fold(root.clone(), |a, s| a.join(s))).ok() };
    stage(rf, o, &have)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Value;

    #[test]
    fn nothing_is_on_by_default() {
        let mut bad: Vec<String> = Vec::new();
        let o: Opts = read(&Settings::default(), &mut bad);
        assert!(bad.is_empty());
        assert_eq!(o, Opts::default());
        assert!(!o.any());
        assert!(o.logic().is_empty());
        assert!(o.notes().is_empty());
    }

    #[test]
    fn each_setting_is_read_and_adds_its_logic_and_note() {
        let mut s: Settings = Settings::default();
        s.set(SKIP, Value::s("story")).unwrap();
        s.set(ANJU, Value::Bool(true)).unwrap();
        s.set(POE, Value::Bool(true)).unwrap();
        let mut bad: Vec<String> = Vec::new();
        let o: Opts = read(&s, &mut bad);
        assert!(bad.is_empty());
        assert_eq!(o, Opts { skip: Skip::Story, anju: true, poe: true });
        assert_eq!(o.logic(), vec![("chicken_count", "1"), ("big_poe_count", "1")]);
        assert_eq!(o.notes().len(), 3);
        s.set(SKIP, Value::s("all")).unwrap();
        assert_eq!(read(&s, &mut bad).skip, Skip::All);
    }

    fn dump() -> Option<std::path::PathBuf> {
        let p: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from("C:/o/nup dump/updates/131072/romfs"));
        if p.is_dir() {
            Some(p)
        } else {
            eprintln!("skipped: no romfs at {}, set OOTRR_ROMFS", p.display());
            None
        }
    }

    fn with(pairs: &[(&str, Value)]) -> Settings {
        let mut s: Settings = Settings::default();
        for (k, v) in pairs {
            s.set(k, v.clone()).unwrap();
        }
        s
    }

    fn built(root: &P, s: &Settings, seed: &str, name: &str) -> std::path::PathBuf {
        let d: crate::slice::data::Data = crate::slice::data::Data::load().unwrap();
        let g: crate::slice::generate::Generated = crate::slice::generate::generate_data(&d, s, seed, &crate::slice::generate::GenOpts::default()).unwrap();
        let load: std::path::PathBuf = std::env::temp_dir().join("ootrr_qol_mods");
        crate::slice::edit::build(&g.seed_file, root, &load, name).unwrap().root.join("romfs")
    }

    fn at(out: &P, r: &str) -> std::path::PathBuf {
        r.split('/').fold(out.to_path_buf(), |a, s| a.join(s))
    }

    #[test]
    fn the_plan_carries_each_option_into_logic_and_notes() {
        use crate::slice::support::plan;
        let p: crate::slice::support::Plan = plan(&Settings::default()).unwrap();
        assert!(!p.qol.any());
        assert!(p.logic().is_empty());
        assert_eq!(p.notes.len(), 2);
        for (k, v, key) in [(ANJU, Value::Bool(true), "chicken_count"), (POE, Value::Bool(true), "big_poe_count")] {
            let p: crate::slice::support::Plan = plan(&with(&[(k, v)])).unwrap();
            assert!(p.qol.any());
            assert_eq!(p.logic(), vec![(key, "1")]);
            assert_eq!(p.notes.len(), 3);
        }
        let p: crate::slice::support::Plan = plan(&with(&[(SKIP, Value::s("all"))])).unwrap();
        assert_eq!(p.qol.skip, Skip::All);
        assert!(p.logic().is_empty());
        assert!(Settings::default().set(SKIP, Value::s("everything")).is_err());
    }

    #[test]
    fn the_logic_override_turns_into_an_int_and_changes_the_two_rules() {
        use crate::logic_items::ItemId;
        use crate::logic_val::Val;
        use crate::slice::data::Data;
        use crate::slice::logic::{spec, world_with};
        use crate::world::{Spec, World};
        let d: Data = Data::load().unwrap();
        let before: Spec = spec(&d, &[]).unwrap();
        let after: Spec = spec(&d, &[("chicken_count", "1"), ("big_poe_count", "1")]).unwrap();
        assert_eq!(before.settings.get("chicken_count"), Some(&Val::Int(7)));
        assert_eq!(after.settings.get("chicken_count"), Some(&Val::Int(1)));
        assert_eq!(after.settings.get("big_poe_count"), Some(&Val::Int(1)));
        let w0: World = world_with(&d, &[("deadly_bonks", "ohko")]).unwrap();
        let w1: World = world_with(&d, &[("deadly_bonks", "ohko"), ("chicken_count", "1"), ("big_poe_count", "1")]).unwrap();
        let rule = |w: &World, n: &str| -> (Vec<ItemId>, String) {
            let r: &crate::rules::Rule = &w.locs[w.loc(n).unwrap()].rule;
            (r.deps.clone(), format!("{:?}", r.node))
        };
        let (a0, n0): (Vec<ItemId>, String) = rule(&w0, "Kak Anju as Child");
        let (a1, n1): (Vec<ItemId>, String) = rule(&w1, "Kak Anju as Child");
        assert!(a1.len() < a0.len(), "{} {}", a1.len(), a0.len());
        assert!(n1.len() < n0.len(), "{n0} {n1}");
        let (_, p0): (Vec<ItemId>, String) = rule(&w0, "Market 10 Big Poes");
        let (_, p1): (Vec<ItemId>, String) = rule(&w1, "Market 10 Big Poes");
        assert!(p0.contains("Has(107, 10)"), "{p0}");
        assert!(p1.contains("Any(") && !p1.contains("Has(107, 10)"), "{p1}");
        let (q, _): (Vec<ItemId>, String) = rule(&world_with(&d, &[]).unwrap(), "Kak Anju as Child");
        assert!(q.is_empty());
    }

    #[test]
    fn seeds_generate_with_the_logic_overrides_and_the_two_toggles_never_change_the_default_seed() {
        use crate::slice::data::Data;
        use crate::slice::generate::{generate_data, GenOpts, Generated};
        let d: Data = Data::load().unwrap();
        let on: Settings = with(&[(ANJU, Value::Bool(true)), (POE, Value::Bool(true)), (SKIP, Value::s("story"))]);
        for seed in ["qol a", "qol b", "qol c"] {
            let a: Generated = generate_data(&d, &Settings::default(), seed, &GenOpts::default()).unwrap();
            let b: Generated = generate_data(&d, &on, seed, &GenOpts::default()).unwrap();
            assert!(!a.seed_file.placements.is_empty());
            assert_eq!(a.seed_file.placements.len(), b.seed_file.placements.len());
        }
        let skip_only: Settings = with(&[(SKIP, Value::s("all"))]);
        let a: Generated = generate_data(&d, &Settings::default(), "qol a", &GenOpts::default()).unwrap();
        let b: Generated = generate_data(&d, &skip_only, "qol a", &GenOpts::default()).unwrap();
        assert_eq!(a.seed_file.placements, b.seed_file.placements);
    }

    #[test]
    fn default_builds_write_none_of_the_option_files() {
        let Some(root) = dump() else {
            return;
        };
        let out: std::path::PathBuf = built(&root, &with(&[("shuffle_rewards", Value::s("off"))]), "qol default", "OOTRR Qol Default Test");
        for f in files() {
            assert!(!at(&out, &f).exists(), "{f}");
        }
        let out: std::path::PathBuf = built(&root, &Settings::default(), "qol default", "OOTRR Qol Rewards Default Test");
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        for f in files() {
            if f == poe::FILE && at(&out, &f).is_file() {
                let z: Vec<u8> = std::fs::read(at(&out, &f)).unwrap();
                let b: crate::bfevfl::Bf = crate::bfevfl::Bf::parse(&rf.zs.decompress(&z).unwrap()).unwrap();
                assert_eq!(poe::sites(&b.flows[0], poe::OLD).len(), poe::SITES);
            } else {
                assert!(!at(&out, &f).exists(), "{f}");
            }
        }
    }

    #[test]
    fn skip_builds_write_the_lock_edits_and_nothing_for_the_title_demos() {
        use crate::byml::{Byml, Node};
        let Some(root) = dump() else {
            return;
        };
        let out: std::path::PathBuf = built(&root, &with(&[(SKIP, Value::s("story"))]), "qol skip", "OOTRR Qol Skip Story Test");
        assert!(!at(&out, skip::FILE).exists());
        for l in skip::LISTS {
            let y: Byml = Byml::parse(&std::fs::read(at(&out, l)).unwrap()).unwrap();
            assert!(y.root.get("Components").unwrap().get(skip::COMPONENT).is_none());
        }
        let out: std::path::PathBuf = built(&root, &with(&[(SKIP, Value::s("all"))]), "qol skip", "OOTRR Qol Skip All Test");
        let y: Byml = Byml::parse(&std::fs::read(at(&out, skip::FILE)).unwrap()).unwrap();
        assert_eq!(y.root.get(skip::KEY), Some(&Node::Bool(false)));
        for l in skip::LISTS {
            assert!(!at(&out, l).exists());
        }
    }

    #[test]
    fn the_anju_build_changes_one_byte_of_one_module() {
        let Some(root) = dump() else {
            return;
        };
        let out: std::path::PathBuf = built(&root, &with(&[(ANJU, Value::Bool(true))]), "qol anju", "OOTRR Qol Anju Test");
        let rf: Romfs = Romfs::open(&root).unwrap();
        let a: Vec<u8> = rf.read(anju::FILE).unwrap();
        let b: Vec<u8> = std::fs::read(at(&out, anju::FILE)).unwrap();
        assert_eq!(a.len(), b.len());
        assert_eq!((0..a.len()).filter(|&i| a[i] != b[i]).count(), 1);
        let m: crate::ainb::Ainb = crate::ainb::Ainb::parse(&b).unwrap();
        assert_eq!(m.in_s32(17, anju::INPUT), Some(anju::NEW));
    }

    #[test]
    fn the_poe_build_merges_with_the_reward_shuffle_instead_of_replacing_it() {
        use crate::bfevfl::{Bf, Flow};
        let Some(root) = dump() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let mut load = |p: &std::path::Path| -> Bf {
            let z: Vec<u8> = std::fs::read(at(p, poe::FILE)).unwrap();
            Bf::parse(&rf.zs.decompress(&z).unwrap()).unwrap()
        };
        let base: Vec<(&str, Value)> = vec![("shuffle_rewards", Value::s("supported")), ("shuffle_dungeon_rewards", Value::Bool(true))];
        let mut on: Vec<(&str, Value)> = base.clone();
        on.push((POE, Value::Bool(true)));
        let a: std::path::PathBuf = built(&root, &with(&base), "qol poe", "OOTRR Qol Poe Off Test");
        let b: std::path::PathBuf = built(&root, &with(&on), "qol poe", "OOTRR Qol Poe On Test");
        let off: Bf = load(&a);
        let onn: Bf = load(&b);
        let (x, y): (&Flow, &Flow) = (&off.flows[0], &onn.flows[0]);
        assert_eq!(poe::sites(x, poe::OLD).len(), poe::SITES);
        assert!(poe::sites(y, poe::OLD).is_empty());
        let diff: Vec<usize> = (0..x.events.len()).filter(|&i| x.events[i] != y.events[i]).collect();
        assert_eq!(diff, vec![242, 243, 244, 246, 248, 250, 252]);
    }


    #[test]
    fn all_three_options_build_together_with_the_other_stagers() {
        use crate::bfevfl::Bf;
        let Some(root) = dump() else {
            return;
        };
        let s: Settings = with(&[
            (SKIP, Value::s("story")),
            (ANJU, Value::Bool(true)),
            (POE, Value::Bool(true)),
            ("shuffle_rewards", Value::s("supported")),
            ("faster_chests", Value::Bool(true)),
            ("skip_intro", Value::Bool(true)),
        ]);
        let out: std::path::PathBuf = built(&root, &s, "qol all", "OOTRR Qol All Test");
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        for f in files() {
            if f == skip::FILE {
                assert!(!at(&out, &f).exists(), "{f}");
            } else {
                assert!(at(&out, &f).is_file(), "{f}");
            }
        }
        let z: Vec<u8> = std::fs::read(at(&out, poe::FILE)).unwrap();
        let b: Bf = Bf::parse(&rf.zs.decompress(&z).unwrap()).unwrap();
        assert!(poe::sites(&b.flows[0], poe::OLD).is_empty());
        assert!(at(&out, crate::intro_skip::FILE).is_file());
    }

    #[test]
    fn the_file_list_is_complete_and_unique() {
        let v: Vec<String> = files();
        assert_eq!(v.len(), 5);
        let mut w: Vec<String> = v.clone();
        w.sort();
        w.dedup();
        assert_eq!(w.len(), v.len());
    }
}
