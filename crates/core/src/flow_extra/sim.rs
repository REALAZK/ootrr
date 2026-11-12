use std::collections::{BTreeMap as BM, HashMap as HM};
use std::path::Path as P;

use crate::bfevfl::{Bf, Flow};
use crate::error::{Error as E, Res};
use crate::flow_extra::apply::{edit_flow, ops_for};
use crate::flow_extra::op::Op;
use crate::flow_locations::{path_of, plan, Change, Choice, Data, Opts as FlowOpts, Req};
use crate::flow_sim::db::Db;
use crate::flow_sim::load::{Disk, Src};
use crate::flow_sim::run::{lines, End, Env, Fin, Mach, Opts, Sim};
use crate::flow_sim::state::{State, V};
use crate::romfs::Romfs;

pub struct Over {
    pub disk: Disk,
    pub flows: HM<String, Flow>,
}

impl Src for Over {
    fn flow(&mut self, name: &str) -> Res<Option<Flow>> {
        match self.flows.get(name) {
            Some(f) => Ok(Some(f.clone())),
            None => self.disk.flow(name),
        }
    }

    fn event(&mut self, name: &str) -> Res<Option<(String, String)>> {
        self.disk.event(name)
    }

    fn cuts(&mut self, name: &str) -> Vec<String> {
        self.disk.cuts(name)
    }
}

pub fn stem(file: &str) -> String {
    file.trim_end_matches(".bfevfl").to_string()
}

pub fn edited(rf: &mut Romfs, location: &str, item: &str) -> Res<HM<String, Flow>> {
    let d: Data = Data::load()?;
    let c: Choice = Choice { location: location.to_string(), item: item.to_string() };
    let (changes, _, _): (Vec<Change>, usize, Vec<String>) = plan(&d, &[c], &FlowOpts::default())?;
    let mut rs: BM<String, Vec<Req>> = BM::new();
    for c in changes.iter().filter(|c| crate::flow_extra::trade::plain(c)) {
        rs.entry(c.file.clone()).or_default().push(Req {
            index: c.index,
            event: c.event.clone(),
            param: c.param.clone(),
            old: c.old.clone(),
            new: c.new.clone(),
        });
    }
    let os: BM<String, Vec<Op>> = ops_for(&changes)?;
    let mut files: Vec<&String> = rs.keys().chain(os.keys()).collect();
    files.sort();
    files.dedup();
    let mut out: HM<String, Flow> = HM::new();
    for file in files {
        let raw: Vec<u8> = rf.read_zs(&path_of(file))?;
        let r: &[Req] = rs.get(file).map(|v| v.as_slice()).unwrap_or(&[]);
        let o: &[Op] = os.get(file).map(|v| v.as_slice()).unwrap_or(&[]);
        let b: Bf = edit_flow(&raw, r, o)?;
        let back: Bf = Bf::parse(&b.write()?)?;
        if back != b {
            return Err(E::Msg(format!("{file} does not survive a byte round trip after the edit")));
        }
        let mut b: Bf = back;
        let f: Flow = b.flows.drain(..).next().ok_or(E::Format("flow extra: the edited file holds no flow"))?;
        out.insert(stem(file), f);
    }
    Ok(out)
}

#[derive(Clone, Debug, Default)]
pub struct Scene {
    pub flow: String,
    pub entry: String,
    pub set: Vec<(String, String)>,
    pub own: Vec<(String, i64)>,
}

pub fn sim(root: &P, flows: HM<String, Flow>) -> Res<Sim> {
    let mut rf: Romfs = Romfs::open(root)?;
    let db: Db = Db::load(&mut rf)?;
    let src: Over = Over { disk: Disk::new(Romfs::open(root)?, None, Vec::new()), flows };
    Ok(Sim::new(db, Box::new(src), Opts::default()))
}

pub fn state(s: &mut Sim, sc: &Scene) -> Res<State> {
    let mut st: State = State::new();
    for (k, v) in &sc.set {
        let h: u32 = s.db.reg(k);
        let x: V = s.db.parse(h, v)?;
        st.v.insert(h, x);
    }
    for (k, n) in &sc.own {
        s.own(&mut st, k, *n);
    }
    s.settle(&mut st);
    Ok(st)
}

pub fn run_from(s: &mut Sim, sc: &Scene, st: State) -> Res<Vec<Fin>> {
    let m: Mach = s.start(&sc.flow, &sc.entry, st, Env::new())?;
    Ok(s.explore(m))
}

pub fn run(s: &mut Sim, sc: &Scene) -> Res<Vec<Fin>> {
    let st: State = state(s, sc)?;
    run_from(s, sc, st)
}

pub fn granted(f: &Fin, item: &str) -> i64 {
    f.m.gr.get(item).copied().unwrap_or(0)
}

pub fn called(f: &Fin, what: &str) -> bool {
    lines(&f.m.log).iter().any(|(_, t)| t.contains(&format!("call {what}")))
}

pub fn broken(fins: &[Fin]) -> usize {
    fins.iter().filter(|f| matches!(f.end, End::Loop(_) | End::Fail(_) | End::Limit)).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf as PB;

    const DUMP: &str = "C:/o/nup dump/updates/131072/romfs";
    const NEW: &str = "Item_Longshot";

    fn root() -> Option<PB> {
        let p: PB = std::env::var_os("OOTRR_ROMFS").map(PB::from).unwrap_or_else(|| PB::from(DUMP));
        if p.join("Event/EventFlow").is_dir() {
            Some(p)
        } else {
            eprintln!("skipped: no romfs at {}, set OOTRR_ROMFS", p.display());
            None
        }
    }

    fn scene(flow: &str, entry: &str, set: &[(&str, &str)], own: &[(&str, i64)]) -> Scene {
        Scene {
            flow: flow.to_string(),
            entry: entry.to_string(),
            set: set.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            own: own.iter().map(|(k, n)| (k.to_string(), *n)).collect(),
        }
    }

    fn both(r: &P, loc: &str, sc: &Scene) -> (Vec<Fin>, Vec<Fin>) {
        let mut rf: Romfs = Romfs::open(r).unwrap();
        let fl: HM<String, Flow> = edited(&mut rf, loc, NEW).unwrap();
        assert!(!fl.is_empty(), "{loc}");
        let mut a: Sim = sim(r, HM::new()).unwrap();
        let mut b: Sim = sim(r, fl).unwrap();
        (run(&mut a, sc).unwrap(), run(&mut b, sc).unwrap())
    }

    fn max(fins: &[Fin], item: &str) -> i64 {
        fins.iter().filter(|f| f.end == End::Done).map(|f| granted(f, item)).max().unwrap_or(0)
    }

    fn min(fins: &[Fin], item: &str) -> i64 {
        fins.iter().map(|f| granted(f, item)).min().unwrap_or(0)
    }

    #[test]
    fn the_vanilla_item_leaves_every_gated_flow_untouched() {
        let Some(r) = root() else { return };
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        for (loc, actor) in [
            ("Kak 20 Gold Skulltula Reward", "Item_AgonyStone"),
            ("DMT Biggoron", "Item_Sword_Biggoron"),
            ("GC Medigoron", "Item_Sword_Giant"),
        ] {
            assert!(edited(&mut rf, loc, actor).unwrap().is_empty(), "{loc}");
        }
        let tiered: HM<String, Flow> = edited(&mut rf, "GC Rolling Goron as Child", "Item_BombBag_Medium").unwrap();
        assert_eq!(tiered.len(), 1);
    }

    #[test]
    fn every_gated_edit_applies_to_the_real_files_and_round_trips() {
        let Some(r) = root() else { return };
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        let x: crate::flow_extra::Extra = crate::flow_extra::Extra::load().unwrap();
        for s in &x.specs {
            let fl: HM<String, Flow> = edited(&mut rf, &s.name, NEW).unwrap();
            for o in &s.ops {
                assert!(fl.contains_key(&stem(&o.file)), "{} {}", s.name, o.file);
            }
        }
    }

    #[test]
    fn kak_20_gives_the_new_item_once_and_skips_the_agony_guide() {
        let Some(r) = root() else { return };
        let sc: Scene = scene(
            "GatheringTokenGold",
            "Reward20",
            &[("KakarikoVillage_GatheringTokenGold_GetReward10", "true"), ("KakarikoVillage_GatheringTokenGold_GetReward20", "false")],
            &[("Item_Token_Gold", 20)],
        );
        let (a, b): (Vec<Fin>, Vec<Fin>) = both(&r, "Kak 20 Gold Skulltula Reward", &sc);
        assert_eq!(max(&a, "Item_AgonyStone"), 1);
        assert!(a.iter().any(|f| called(f, "ActionGuide:OpenActionGuideSceen")));
        assert_eq!(max(&b, NEW), 1);
        assert_eq!(max(&b, "Item_AgonyStone"), 0);
        assert!(!b.iter().any(|f| called(f, "ActionGuide:OpenActionGuideSceen")));
        assert!(b.iter().filter(|f| granted(f, NEW) == 1).all(|f| f.m.writes.get("KakarikoVillage_GatheringTokenGold_GetReward20").map(|s| s.as_str()) == Some("true")));
        assert_eq!(broken(&b), broken(&a));
        let done: Scene = scene("GatheringTokenGold", "Reward20", &[("KakarikoVillage_GatheringTokenGold_GetReward20", "true")], &[("Item_Token_Gold", 20)]);
        let (_, again): (Vec<Fin>, Vec<Fin>) = both(&r, "Kak 20 Gold Skulltula Reward", &done);
        assert_eq!(max(&again, NEW), 0);
        let hundred: Scene = scene("GatheringTokenGold", "Reward100", &[], &[("Item_Token_Gold", 100)]);
        let (va, vb): (Vec<Fin>, Vec<Fin>) = both(&r, "Kak 20 Gold Skulltula Reward", &hundred);
        assert!(va.iter().any(|f| called(f, "ActionGuide:OpenActionGuideSceen")));
        assert!(!vb.iter().any(|f| called(f, "ActionGuide:OpenActionGuideSceen")));
    }

    #[test]
    fn rolling_goron_gives_the_item_without_a_bomb_bag() {
        let Some(r) = root() else { return };
        let sc: Scene = scene("Npc_Goron_006_Child", "Talk", &[("GoronCity_StopGoron006", "true"), ("GoronCity_GetBombBag_fromGoron006", "false")], &[]);
        let (a, b): (Vec<Fin>, Vec<Fin>) = both(&r, "GC Rolling Goron as Child", &sc);
        assert_eq!(max(&a, "Item_BombBag_Medium") + max(&a, "Item_BombBag_Large"), 0);
        assert_eq!(max(&b, NEW), 1);
        assert!(b.iter().filter(|f| granted(f, NEW) == 1).all(|f| f.m.writes.contains_key("GoronCity_GetBombBag_fromGoron006")));
        let bag: Scene = scene("Npc_Goron_006_Child", "Talk", &[("GoronCity_StopGoron006", "true")], &[("Item_BombBag_Small", 1)]);
        let (a, b): (Vec<Fin>, Vec<Fin>) = both(&r, "GC Rolling Goron as Child", &bag);
        assert_eq!(max(&a, "Item_BombBag_Medium"), 1);
        assert_eq!(max(&b, NEW), 1);
        assert_eq!(max(&b, "Item_BombBag_Medium"), 0);
        assert_eq!(broken(&b), broken(&a));
    }

    #[test]
    fn biggoron_keeps_the_knife_and_skips_the_sword_equip_loop() {
        let Some(r) = root() else { return };
        let sc: Scene = scene("Npc_Goron_Biggoron_Adult", "SubEvent_GetSwordBigGoron_Step11", &[("Step_SubEvent_GetSwordBigGoron", "Step11")], &[("Weapon_Sword_Giant", 1)]);
        let (a, b): (Vec<Fin>, Vec<Fin>) = both(&r, "DMT Biggoron", &sc);
        assert_eq!(min(&a, "Weapon_Sword_Giant"), -1);
        assert!(a.iter().any(|f| called(f, "Dm_SY_GetInEvent:GetInEventItemSwordBiggoron")));
        assert_eq!(min(&b, "Weapon_Sword_Giant"), 0);
        assert_eq!(max(&b, NEW), 1);
        assert_eq!(max(&b, "Item_Sword_Biggoron"), 0);
        assert!(b.iter().any(|f| called(f, "Dm_SY_GetInEvent:GetInEvent_NotTurnForPosCamera_BigGoron")));
        assert!(!b.iter().any(|f| called(f, "Dm_SY_GetInEvent:GetInEventItemSwordBiggoron")));
        assert!(b.iter().filter(|f| granted(f, NEW) == 1).all(|f| f.m.reqs.iter().any(|q| q.starts_with("SubEvent_GetSwordBigGoron -> Complete"))));
        assert!(broken(&b) <= broken(&a));
        let broken_knife: Scene = scene("Npc_Goron_Biggoron_Adult", "SubEvent_GetSwordBigGoron_Step11", &[("Step_SubEvent_GetSwordBigGoron", "Step11")], &[("Weapon_Sword_BrokenGiant", 1)]);
        let (_, b): (Vec<Fin>, Vec<Fin>) = both(&r, "DMT Biggoron", &broken_knife);
        assert_eq!(min(&b, "Weapon_Sword_BrokenGiant"), 0);
        assert_eq!(max(&b, NEW), 1);
    }

    #[test]
    fn medigoron_sells_once_even_with_the_biggoron_sword() {
        let Some(r) = root() else { return };
        let flag: &str = "HaveTalked_Special.Npc_Goron_Medigoron_Adult_Talk_Event26";
        let sc: Scene = scene("Npc_Goron_Medigoron_Adult", "Talk", &[("PlayerStatus.CurrentRupee", "300")], &[("Weapon_Sword_Biggoron", 1), ("Weapon_Sword_BrokenGiant", 1)]);
        let (a, b): (Vec<Fin>, Vec<Fin>) = both(&r, "GC Medigoron", &sc);
        assert!(a.iter().all(|f| f.m.gr.is_empty()));
        assert_eq!(max(&b, NEW), 1);
        assert_eq!(min(&b, "Weapon_Sword_BrokenGiant"), 0);
        let sold: Vec<&Fin> = b.iter().filter(|f| granted(f, NEW) == 1).collect();
        assert!(!sold.is_empty());
        assert!(sold.iter().all(|f| f.m.writes.get(flag).map(|s| s.as_str()) == Some("true")));
        assert!(b.iter().filter(|f| granted(f, NEW) == 0).all(|f| !f.m.writes.contains_key(flag)));
        let mut rf: Romfs = Romfs::open(&r).unwrap();
        let mut s: Sim = sim(&r, edited(&mut rf, "GC Medigoron", NEW).unwrap()).unwrap();
        let after: State = sold[0].m.st.clone();
        let again: Vec<Fin> = run_from(&mut s, &sc, after).unwrap();
        assert_eq!(max(&again, NEW), 0);
        let plain: Scene = scene("Npc_Goron_Medigoron_Adult", "Talk", &[("PlayerStatus.CurrentRupee", "300")], &[]);
        let (a, b): (Vec<Fin>, Vec<Fin>) = both(&r, "GC Medigoron", &plain);
        assert!(a.iter().any(|f| !f.m.gr.is_empty()));
        assert_eq!(max(&b, NEW), 1);
        assert_eq!(broken(&b), broken(&a));
    }
}
