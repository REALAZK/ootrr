use crate::byml::Byml;
use crate::error::Res;
use crate::flags::FlagList;
use crate::flags_gen;
use crate::flags_harvest::{Evidence, Harvest, Mem};
use crate::flags_keyed;
use crate::flags_map;
use crate::flags_rel;
use crate::flags_scan::Walked;
use crate::romfs::Romfs;

pub const LIST_PATH: &str = "GameData/GameDataList.Product.110.byml.zs";

pub struct Built {
    pub list: FlagList,
    pub members: Mem,
    pub walked: Walked,
    pub tested: u64,
    pub strings: u64,
    pub stages: Vec<(String, usize)>,
    pub evidence: Evidence,
}

pub fn load(rf: &mut Romfs) -> Res<FlagList> {
    let y: Byml = rf.read_byml(LIST_PATH)?;
    FlagList::from_byml(&y)
}

fn settle(l: &mut FlagList, mn: &mut Mem, stages: &mut Vec<(String, usize)>) {
    let mut seen: usize = usize::MAX;
    for round in 0..8 {
        let a: usize = flags_rel::structs(l, mn);
        let b: usize = flags_rel::exps(l);
        let c: usize = if mn.len() != seen {
            flags_gen::run(l, mn)
        } else {
            0
        };
        let d: usize = flags_gen::siblings(l);
        seen = mn.len();
        stages.push((format!("relations round {}", round + 1), l.named()));
        if a + b + c + d == 0 {
            break;
        }
    }
}

pub fn build(rf: &mut Romfs) -> Res<Built> {
    let mut list: FlagList = load(rf)?;
    let mut stages: Vec<(String, usize)> = Vec::new();
    let mut h: Harvest = Harvest::new(&list);
    h.want(flags_map::wanted());
    let walked: Walked = h.run(rf, &mut list)?;
    stages.push(("string harvest".to_string(), list.named()));
    let mut members: Mem = std::mem::take(&mut h.members);
    settle(&mut list, &mut members, &mut stages);
    let k: flags_keyed::Keys = flags_keyed::collect(rf)?;
    flags_keyed::label(&mut list, &k);
    stages.push(("placement keys".to_string(), list.named()));
    Ok(Built {
        list,
        members,
        walked,
        tested: h.tested,
        strings: h.strings,
        stages,
        evidence: h.evidence,
    })
}
