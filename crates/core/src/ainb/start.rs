use super::grow::CONST;
use super::node::{guid, new_node, In};
use super::tab::{INSZ, OUTSZ, PSZ};
use super::{Ainb, Input, Plug};
use crate::error::{Error, Res};
use crate::flags::{hash_name, FlagList};

pub const PUSH: &str = "SeqOneShotPushToPouch";
pub const SET_INT: &str = "SeqOneShotSetGameDataInt";
pub const SET_FLAG: &str = "SeqOneShotSetNonStructGameDataBool";
const LIVE: u32 = 0x80_0000;
const RESET_ON_BANC: i64 = 0x101;
pub const MODULE: &str = "Sequence/SetPouchAndStatusForGameStart.module.ainb";
pub const ACTOR_DIR: &str = "Work/Actor/";
pub const ACTOR_EXT: &str = ".engine__actor__ActorParam.gyml";
pub const QUARTERS: i32 = 4;
const CHILD: usize = 2;

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub el: usize,
    pub actor: String,
    pub num: i32,
}

pub fn actor_path(n: &str) -> String {
    if n.contains('/') {
        return n.to_string();
    }
    if n.ends_with(".gyml") {
        return format!("{ACTOR_DIR}{n}");
    }
    format!("{ACTOR_DIR}{n}{ACTOR_EXT}")
}

pub fn items(a: &Ainb) -> Vec<Item> {
    let mut out: Vec<Item> = Vec::new();
    for i in a.find_els(PUSH) {
        let actor: String = a.in_str(i, "ActorName").unwrap_or("").to_string();
        out.push(Item { el: i, actor, num: a.in_s32(i, "Num").unwrap_or(1) });
    }
    out
}

fn int_el(a: &Ainb, name: &str) -> Option<usize> {
    a.find_els(SET_INT).into_iter().find(|&i| a.in_str(i, "GameDataName") == Some(name))
}

pub fn game_int(a: &Ainb, name: &str) -> Option<i32> {
    a.in_s32(int_el(a, name)?, "Value")
}

pub fn set_game_int(a: &mut Ainb, name: &str, v: i32) -> Res<()> {
    let i: usize = int_el(a, name).ok_or(Error::Format("ainb: no such starting value"))?;
    let g: usize = a.find_in(i, 0, "Value").ok_or(Error::Format("ainb: starting value has no Value input"))?;
    let x: Input = a.input(0, g).ok_or(Error::Format("ainb: input out of range"))?;
    if x.src != -1 {
        return Err(Error::Format("ainb: starting value is not a constant"));
    }
    a.set_in_val(0, g, &v.to_le_bytes())
}

pub fn hearts(a: &Ainb) -> Option<i32> {
    Some(game_int(a, "MaxLife")? / QUARTERS)
}

pub fn set_hearts(a: &mut Ainb, n: i32) -> Res<()> {
    if !(1..=100).contains(&n) {
        return Err(Error::Format("ainb: hearts must be between 1 and 100"));
    }
    set_game_int(a, "MaxLife", n * QUARTERS)?;
    set_game_int(a, "Life", n * QUARTERS)
}

fn take(v: &[u8], b: usize, n: usize, sz: usize) -> Res<Vec<u8>> {
    Ok(v.get(b * sz..(b + n) * sz).ok_or(Error::Format("ainb: entry range out of bounds"))?.to_vec())
}

fn clone_el(a: &mut Ainb, t: usize, seed: &str) -> Res<usize> {
    let n: usize = a.els.len();
    if n >= 0xffff || !a.params[t].plugs.is_empty() {
        return Err(Error::Format("ainb: element cannot be cloned"));
    }
    let mut e: [u8; super::EL] = a.els[t];
    e[2..4].copy_from_slice(&(n as u16).to_le_bytes());
    e[0x2c..0x3c].copy_from_slice(&guid(seed));
    let mut p = a.params[t].clone();
    for k in 0..6 {
        let (b, c): (usize, usize) = (p.pe[k][0] as usize, p.pe[k][1] as usize);
        let d: Vec<u8> = take(&a.props[k], b, c, PSZ[k])?;
        p.pe[k][0] = (a.props[k].len() / PSZ[k]) as u32;
        a.props[k].extend(d);
        let (ib, ic, ob, oc): (usize, usize, usize, usize) = (p.ioe[k][0] as usize, p.ioe[k][1] as usize, p.ioe[k][2] as usize, p.ioe[k][3] as usize);
        let di: Vec<u8> = take(&a.ins[k], ib, ic, INSZ[k])?;
        let dout: Vec<u8> = take(&a.outs[k], ob, oc, OUTSZ[k])?;
        p.ioe[k][0] = (a.ins[k].len() / INSZ[k]) as u32;
        p.ioe[k][2] = (a.outs[k].len() / OUTSZ[k]) as u32;
        a.ins[k].extend(di);
        a.outs[k].extend(dout);
    }
    a.els.push(e);
    a.params.push(p);
    Ok(n)
}

pub fn add_item(a: &mut Ainb, name: &str, num: i32) -> Res<usize> {
    if !(1..=999).contains(&num) {
        return Err(Error::Format("ainb: item count must be between 1 and 999"));
    }
    let path: String = actor_path(name);
    let mut found: Option<(usize, usize, usize)> = None;
    for t in a.find_els(PUSH) {
        for p in 0..a.els.len() {
            if let Some(j) = a.params[p].group(CHILD).iter().position(|g| g.a as usize == t) {
                found = Some((t, p, j));
            }
        }
    }
    let (t, p, j): (usize, usize, usize) = found.ok_or(Error::Format("ainb: no push to pouch node to copy"))?;
    let tpl: Plug = a.params[p].group(CHILD)[j].clone();
    let id: usize = clone_el(a, t, &format!("{path}#{}", a.els.len()))?;
    let off: u32 = a.pool.add(&path);
    let g: usize = a.find_in(id, 3, "ActorName").ok_or(Error::Format("ainb: node has no ActorName"))?;
    a.set_in_val(3, g, &off.to_le_bytes())?;
    let c: usize = a.find_in(id, 0, "Num").ok_or(Error::Format("ainb: node has no Num"))?;
    a.set_in_val(0, c, &num.to_le_bytes())?;
    a.params[p].add_plug(CHILD, j + 1, Plug { a: id as u32, b: tpl.b, extra: tpl.extra })?;
    Ok(id)
}

pub fn flags(a: &Ainb) -> Vec<(String, bool)> {
    let mut out: Vec<(String, bool)> = Vec::new();
    for i in a.find_els(SET_FLAG) {
        let n: String = a.in_str(i, "GameDataFullName").unwrap_or("").to_string();
        let v: bool = a.find_in(i, 1, "Value").and_then(|g| a.input(1, g)).is_some_and(|x| x.val.iter().any(|&b| b != 0));
        out.push((n, v));
    }
    out
}

fn flag_el(a: &Ainb, name: &str) -> Option<usize> {
    a.find_els(SET_FLAG).into_iter().find(|&i| a.in_str(i, "GameDataFullName") == Some(name))
}

pub fn add_flag(a: &mut Ainb, name: &str, v: bool) -> Res<usize> {
    if name.is_empty() || name.contains('\0') {
        return Err(Error::Format("ainb: bad flag name"));
    }
    if let Some(i) = flag_el(a, name) {
        let g: usize = a.find_in(i, 1, "Value").ok_or(Error::Format("ainb: flag node has no Value input"))?;
        a.set_in_val(1, g, &u32::from(v).to_le_bytes())?;
        return Ok(i);
    }
    let anchor: usize = *a.find_els(SET_INT).first().ok_or(Error::Format("ainb: no starting value node to anchor on"))?;
    let mut found: Option<(usize, usize)> = None;
    for p in 0..a.els.len() {
        if let Some(j) = a.params[p].group(CHILD).iter().position(|g| g.a as usize == anchor) {
            found = Some((p, j));
        }
    }
    let (p, j): (usize, usize) = found.ok_or(Error::Format("ainb: starting value node has no parent"))?;
    let tpl: Plug = a.params[p].group(CHILD)[j].clone();
    let off: u32 = a.pool.add(name);
    let ins: [In; 3] = [
        In { k: 0, name: "GameDataIndex", flags: CONST, val: (-1i32).to_le_bytes().to_vec() },
        In { k: 1, name: "Value", flags: LIVE, val: u32::from(v).to_le_bytes().to_vec() },
        In { k: 3, name: "GameDataFullName", flags: LIVE, val: off.to_le_bytes().to_vec() },
    ];
    let id: usize = new_node(a, SET_FLAG, &format!("{name}#{}", a.els.len()), &ins)?;
    let at: usize = a.params[p].group(CHILD).len();
    a.params[p].add_plug(CHILD, at, Plug { a: id as u32, b: tpl.b, extra: tpl.extra })?;
    Ok(id)
}

pub fn check_flag(l: &FlagList, name: &str) -> Res<()> {
    let f: &crate::flags::Flag = l.by_hash(hash_name(name) as u64).ok_or(Error::Format("ainb: flag is not in the game data list"))?;
    if f.ty != "Bool" {
        return Err(Error::Format("ainb: flag is not a Bool"));
    }
    if f.reset & RESET_ON_BANC != 0 {
        return Err(Error::Format("ainb: flag is reset on every scene change"));
    }
    Ok(())
}

pub fn set_flags(a: &mut Ainb, f: &[(String, bool)]) -> Res<()> {
    for (n, v) in f {
        add_flag(a, n, *v)?;
    }
    Ok(())
}

pub fn patch(a: &mut Ainb, h: Option<i32>, add: &[(String, i32)]) -> Res<()> {
    if let Some(n) = h {
        set_hearts(a, n)?;
    }
    for (name, num) in add {
        add_item(a, name, *num)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ainb::audit;
    use crate::ainb::tests::synth;

    #[test]
    fn reads_the_starting_state() {
        let a: Ainb = synth();
        assert_eq!(hearts(&a), Some(3));
        assert_eq!(game_int(&a, "Life"), Some(12));
        let v: Vec<Item> = items(&a);
        assert_eq!(v.len(), 1);
        assert!(v[0].actor.ends_with("Item_Synthetic_A.engine__actor__ActorParam.gyml"));
    }

    #[test]
    fn hearts_change_is_in_place() {
        let a: Ainb = synth();
        let before: Vec<u8> = a.write().unwrap();
        let mut b: Ainb = a.clone();
        set_hearts(&mut b, 7).unwrap();
        let after: Vec<u8> = b.write().unwrap();
        assert_eq!(before.len(), after.len());
        let d: Vec<usize> = (0..before.len()).filter(|&i| before[i] != after[i]).collect();
        assert_eq!(d.len(), 2);
        let c: Ainb = Ainb::parse(&after).unwrap();
        assert_eq!(hearts(&c), Some(7));
        assert_eq!(game_int(&c, "Life"), Some(28));
    }

    #[test]
    fn hearts_bounds_and_missing_values() {
        let mut a: Ainb = synth();
        assert!(set_hearts(&mut a, 0).is_err());
        assert!(set_hearts(&mut a, 101).is_err());
        assert!(set_game_int(&mut a, "Nope", 1).is_err());
    }

    #[test]
    fn adds_an_item() {
        let mut a: Ainb = synth();
        let id: usize = add_item(&mut a, "Item_Synthetic_B", 5).unwrap();
        let b: Vec<u8> = a.write().unwrap();
        let c: Ainb = Ainb::parse(&b).unwrap();
        assert_eq!(Ainb::parse(&c.write().unwrap()).unwrap(), c);
        assert!(audit(&c).is_empty());
        let v: Vec<Item> = items(&c);
        assert_eq!(v.len(), 2);
        assert_eq!(v[1].el, id);
        assert_eq!(v[1].num, 5);
        assert_eq!(v[1].actor, actor_path("Item_Synthetic_B"));
        let kids: Vec<u32> = c.params[0].group(CHILD).iter().map(|g| g.a).collect();
        assert_eq!(kids, vec![1, id as u32, 2, 3]);
    }

    #[test]
    fn adds_the_same_item_twice() {
        let mut a: Ainb = synth();
        add_item(&mut a, "Item_Synthetic_B", 1).unwrap();
        add_item(&mut a, "Item_Synthetic_B", 1).unwrap();
        let c: Ainb = Ainb::parse(&a.write().unwrap()).unwrap();
        assert_eq!(items(&c).len(), 3);
        assert_ne!(c.els[4][0x2c..0x3c], c.els[5][0x2c..0x3c]);
        let n: usize = c.pool.b.windows(16).filter(|w| *w == b"Item_Synthetic_B").count();
        assert_eq!(n, 1);
    }

    #[test]
    fn rejects_bad_counts() {
        let mut a: Ainb = synth();
        assert!(add_item(&mut a, "X", 0).is_err());
        assert!(add_item(&mut a, "X", 1000).is_err());
    }

    #[test]
    fn actor_names_expand() {
        assert_eq!(actor_path("A"), "Work/Actor/A.engine__actor__ActorParam.gyml");
        assert_eq!(actor_path("A.engine__actor__ActorParam.gyml"), "Work/Actor/A.engine__actor__ActorParam.gyml");
        assert_eq!(actor_path("Dir/A"), "Dir/A");
    }

    #[test]
    fn adds_a_flag_setter() {
        let mut a: Ainb = synth();
        let id: usize = add_flag(&mut a, "Some_Flag", false).unwrap();
        let c: Ainb = Ainb::parse(&a.write().unwrap()).unwrap();
        assert_eq!(Ainb::parse(&c.write().unwrap()).unwrap(), c);
        assert!(audit(&c).is_empty());
        assert_eq!(flags(&c), vec![("Some_Flag".to_string(), false)]);
        assert_eq!(c.el_name(id), SET_FLAG);
        assert_eq!(c.el_hash(id), crate::murmur::murmur3_32(SET_FLAG.as_bytes(), 0));
        let kids: Vec<u32> = c.params[0].group(CHILD).iter().map(|g| g.a).collect();
        assert_eq!(kids, vec![1, 2, 3, id as u32]);
        assert_eq!(items(&c).len(), 1);
        assert_eq!(game_int(&c, "Life"), Some(12));
    }

    #[test]
    fn a_flag_twice_updates_in_place() {
        let mut a: Ainb = synth();
        let x: usize = add_flag(&mut a, "Some_Flag", false).unwrap();
        let n: usize = a.els.len();
        assert_eq!(add_flag(&mut a, "Some_Flag", true).unwrap(), x);
        assert_eq!(a.els.len(), n);
        add_flag(&mut a, "Other_Flag", true).unwrap();
        let c: Ainb = Ainb::parse(&a.write().unwrap()).unwrap();
        assert!(audit(&c).is_empty());
        assert_eq!(flags(&c), vec![("Some_Flag".to_string(), true), ("Other_Flag".to_string(), true)]);
        assert_ne!(c.els[x][0x2c..0x3c], c.els[x + 1][0x2c..0x3c]);
    }

    #[test]
    fn flag_names_are_checked() {
        let mut a: Ainb = synth();
        assert!(add_flag(&mut a, "", true).is_err());
        assert!(add_flag(&mut a, "A\0B", true).is_err());
        let mut b: Ainb = synth();
        b.params[0].plugs.clear();
        b.params[0].pl = [[0; 2]; 10];
        assert!(add_flag(&mut b, "X", true).is_err());
    }

    #[test]
    fn flag_checks_use_the_game_data_list() {
        use crate::flags::{Body, Flag};
        use crate::json::Json;
        let f = |n: &str, ty: &str, reset: i64| Flag {
            hash: hash_name(n) as u64,
            ty: ty.to_string(),
            def: Json::Bool(true),
            reset,
            save: 0,
            extra: None,
            size: None,
            body: Body::Plain,
            name: Some(n.to_string()),
            how: None,
        };
        let l: FlagList = FlagList::from_flags(vec![f("Keep", "Bool", 8), f("Gone", "Bool", 9), f("Warp", "Bool", 256), f("Num", "Int", 8)]);
        assert!(check_flag(&l, "Keep").is_ok());
        assert!(check_flag(&l, "Gone").is_err());
        assert!(check_flag(&l, "Warp").is_err());
        assert!(check_flag(&l, "Num").is_err());
        assert!(check_flag(&l, "Missing").is_err());
    }
}
