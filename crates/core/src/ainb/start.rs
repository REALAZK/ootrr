use super::tab::{INSZ, OUTSZ, PSZ};
use super::{Ainb, Input, Plug};
use crate::error::{Error, Res};
use crate::murmur::murmur3_32;

pub const PUSH: &str = "SeqOneShotPushToPouch";
pub const SET_INT: &str = "SeqOneShotSetGameDataInt";
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

fn guid(seed: &str) -> [u8; 16] {
    let mut g: [u8; 16] = [0; 16];
    for j in 0..4 {
        g[4 * j..4 * j + 4].copy_from_slice(&murmur3_32(seed.as_bytes(), j as u32 + 1).to_le_bytes());
    }
    g
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
}
