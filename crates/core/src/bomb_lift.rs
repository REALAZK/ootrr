use crate::byml::{Byml, Node};
use crate::error::Res;
use crate::romfs::Romfs;
use crate::sarc;
use crate::staging::{rewrite, Out};

pub const LOW: u32 = 3655598229;
pub const NONE: u32 = 3069107721;
pub const KEY: &str = "LiftGrade";
pub const PARAM: &str = "Component/LiftableParam/";

pub const PACKS: [&str; 4] = [
    "Item_Bomb",
    "Obj_BombFlower_A_01",
    "Obj_BombFlower_A_04",
    "Obj_BombFlower_Fruit_For_Break_Obj_DeathMtTrail_BrokenRock_A_01",
];

pub fn pack_of(name: &str) -> String {
    format!("Pack/Actor/{name}.pack.zs")
}

pub fn lower(raw: &[u8]) -> Res<(Vec<u8>, usize)> {
    let mut es: Vec<sarc::Entry> = sarc::parse(raw)?;
    let mut n: usize = 0;
    for e in es.iter_mut().filter(|e| e.name.starts_with(PARAM)) {
        let mut y: Byml = Byml::parse(&e.data)?;
        if y.root.get(KEY) == Some(&Node::U32(LOW)) {
            y.root.set(KEY, Node::U32(NONE));
            e.data = y.write()?;
            n += 1;
        }
    }
    Ok((sarc::write(&es)?, n))
}

pub fn stage(rf: &mut Romfs) -> Res<Vec<Out>> {
    let mut outs: Vec<Out> = Vec::new();
    for p in PACKS {
        outs.push(rewrite(rf, &pack_of(p), |raw| lower(raw).map(|(b, _)| b))?);
    }
    Ok(outs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn param(grade: u32) -> Vec<u8> {
        let y: Byml = Byml {
            root: Node::Map(vec![(KEY.to_string(), Node::U32(grade)), ("LiftableType".to_string(), Node::U32(7))]),
            version: 2,
            big: false,
        };
        y.write().unwrap()
    }

    #[test]
    fn only_the_low_grade_entries_drop_to_none_and_the_rest_stays() {
        let es: Vec<sarc::Entry> = vec![
            sarc::Entry { name: format!("{PARAM}A.bgyml"), data: param(LOW) },
            sarc::Entry { name: format!("{PARAM}B.bgyml"), data: param(5) },
            sarc::Entry { name: "Other/File.bin".to_string(), data: vec![1, 2, 3] },
        ];
        let raw: Vec<u8> = sarc::write(&es).unwrap();
        let (out, n) = lower(&raw).unwrap();
        assert_eq!(n, 1);
        let back: Vec<sarc::Entry> = sarc::parse(&out).unwrap();
        let a: Byml = Byml::parse(&sarc::get(&back, &format!("{PARAM}A.bgyml")).unwrap().data).unwrap();
        let b: Byml = Byml::parse(&sarc::get(&back, &format!("{PARAM}B.bgyml")).unwrap().data).unwrap();
        assert_eq!(a.root.get(KEY), Some(&Node::U32(NONE)));
        assert_eq!(a.root.get("LiftableType"), Some(&Node::U32(7)));
        assert_eq!(b.root.get(KEY), Some(&Node::U32(5)));
        assert_eq!(sarc::get(&back, "Other/File.bin").unwrap().data, vec![1, 2, 3]);
    }
}
