use crate::bfevfl::flow::NONE;
use crate::bfevfl::{Case, Cont, Entry, Event, Flow, Kind, Val};
use crate::error::{Error as E, Res};

pub const FILE: &str = "Enemy_Scrub_Business_Common.bfevfl";
const POUCH: &str = "SimpleShopRoot_Enemy_Scrub_Business_Pouch";
const ONE: &str = "SimpleShopSub_Scrub_Business_OneTime";
const SALE: (usize, &str) = (155, "Event293");
const ROWS: [(usize, &str); 3] = [(327, "Event578"), (172, "Event6"), (171, "Event309")];
const LEAVE: (usize, &str) = (154, "Event305");
const HEART: &str = "Enemy_Scrub_Business_Heart_02";
const HOLDER: &str = "GameDataActor";
const ASK: &str = "EventQueryIsGameDataBool";
const SET: &str = "EventTriggerSetGameDataBool";
const BOUGHT: &str = "ScrubBusiness_BuySuccess";
const ARGS: [(&str, &str); 3] = [("ActorName", "SaleActor"), ("Price", "SalePrice"), ("SellerMessageId", "SaleMsg")];

#[derive(Clone, Debug, PartialEq)]
pub struct Sale {
    pub entry: String,
    pub flag: String,
    pub actor: String,
    pub price: i32,
}

fn idx(n: usize) -> Res<u16> {
    u16::try_from(n).map_err(|_| E::Format("scrubs_b: index does not fit in 16 bits"))
}

fn call<'a>(f: &'a mut Flow, at: (usize, &str), entry: &str) -> Res<&'a mut Cont> {
    let e: &mut Event = f.events.get_mut(at.0).ok_or_else(|| E::Msg(format!("event index {} is out of range", at.0)))?;
    if e.name != at.1 {
        return Err(E::Msg(format!("event {} is named {} and not {}, the game files differ", at.0, e.name, at.1)));
    }
    let Kind::Sub { flow, entry: en, params, .. } = &mut e.kind else {
        return Err(E::Msg(format!("event {} is not a sub flow call, the game files differ", at.1)));
    };
    if !flow.is_empty() || en != entry {
        return Err(E::Msg(format!("event {} calls {flow}:{en} and not :{entry}, the game files differ", at.1)));
    }
    params.as_mut().ok_or_else(|| E::Msg(format!("event {} has no parameters", at.1)))
}

fn slot<'a>(c: &'a mut Cont, k: &str) -> Res<&'a mut Val> {
    c.items.iter_mut().find(|(n, _)| n == k).map(|(_, v)| v).ok_or_else(|| E::Msg(format!("the sale event has no {k} parameter, the game files differ")))
}

pub fn base(f: &mut Flow) -> Res<()> {
    let now: Vec<Val> = {
        let c: &mut Cont = call(f, SALE, ONE)?;
        let mut v: Vec<Val> = Vec::new();
        for (k, _) in ARGS {
            let x: &Val = slot(c, k)?;
            match (k, x) {
                ("Price", Val::Int(_)) => v.push(x.clone()),
                ("Price", _) => return Err(E::Msg("the sale event price is not a number, the base edit was already applied or the game files differ".to_string())),
                (_, Val::Str(_)) => v.push(x.clone()),
                _ => return Err(E::Msg(format!("the sale event {k} is not a name, the base edit was already applied or the game files differ"))),
            }
        }
        v
    };
    for r in ROWS {
        let c: &mut Cont = call(f, r, POUCH)?;
        if ARGS.iter().any(|(_, a)| c.get(a).is_some()) {
            return Err(E::Msg(format!("event {} already passes the sale arguments", r.1)));
        }
        for ((_, a), x) in ARGS.iter().zip(&now) {
            c.items.push((a.to_string(), x.clone()));
        }
    }
    let c: &mut Cont = call(f, SALE, ONE)?;
    for (k, a) in ARGS {
        *slot(c, k)? = Val::Arg(a.to_string());
    }
    Ok(())
}

fn find(f: &Flow, query: bool, name: &str) -> Res<(u16, u16)> {
    let i: usize = f
        .actors
        .iter()
        .position(|a| a.name == HOLDER && a.sub.is_empty())
        .ok_or_else(|| E::Msg(format!("the scrub flow has no {HOLDER}, the game files differ")))?;
    let list: &Vec<String> = if query { &f.actors[i].queries } else { &f.actors[i].actions };
    let j: usize = list.iter().position(|x| x == name).ok_or_else(|| E::Msg(format!("{HOLDER} has no {name}, the game files differ")))?;
    Ok((idx(i)?, idx(j)?))
}

fn push(f: &mut Flow, kind: Kind) -> Res<u16> {
    let i: u16 = idx(f.events.len())?;
    f.events.push(Event { name: format!("Scrub{i}"), kind });
    Ok(i)
}

fn flag(name: &str) -> Cont {
    Cont { items: vec![("Index".to_string(), Val::Int(-1)), ("GameDataName".to_string(), Val::Str(name.to_string()))] }
}

pub fn add(f: &mut Flow, s: &Sale, label: &str) -> Res<()> {
    if f.entries.iter().any(|e| e.name == s.entry) {
        return Err(E::Msg(format!("the scrub flow already has an entry {}", s.entry)));
    }
    let leave: &Event = f.events.get(LEAVE.0).ok_or(E::Format("scrubs_b: the leave event is out of range"))?;
    if leave.name != LEAVE.1 || !matches!(leave.kind, Kind::Action { .. }) {
        return Err(E::Msg(format!("event {} is not the scrub leave action {}, the game files differ", LEAVE.0, LEAVE.1)));
    }
    let pad: u16 = f.entries.iter().find(|e| e.name == HEART).map(|e| e.pad).ok_or_else(|| E::Msg(format!("the scrub flow has no {HEART} entry")))?;
    let greet: Vec<(String, Val)> = {
        let c: &mut Cont = call(f, ROWS[0], POUCH)?;
        c.items.iter().filter(|(k, _)| k.starts_with("Greeting")).cloned().collect()
    };
    if greet.len() != 3 {
        return Err(E::Msg(format!("event {} does not pass three greetings, the game files differ", ROWS[0].1)));
    }
    let (qa, qi): (u16, u16) = find(f, true, ASK)?;
    let (sa, si): (u16, u16) = find(f, false, SET)?;
    let mut sp: Cont = flag(&s.flag);
    sp.items.insert(1, ("Value".to_string(), Val::Bool(1)));
    let set: u16 = push(f, Kind::Action { next: NONE, actor: sa, act: si, params: Some(sp) })?;
    let chk: u16 = push(f, Kind::Switch { actor: qa, query: qi, cases: vec![Case { val: 0, ev: set }], params: Some(flag(BOUGHT)) })?;
    let mut items: Vec<(String, Val)> = greet;
    items.push(("SaleActor".to_string(), Val::Str(s.actor.clone())));
    items.push(("SalePrice".to_string(), Val::Int(s.price)));
    items.push(("SaleMsg".to_string(), Val::Str(label.to_string())));
    let sub: u16 = push(f, Kind::Sub { next: chk, params: Some(Cont { items }), flow: String::new(), entry: POUCH.to_string() })?;
    let gate: u16 = push(f, Kind::Switch { actor: qa, query: qi, cases: vec![Case { val: 0, ev: idx(LEAVE.0)? }, Case { val: 1, ev: sub }], params: Some(flag(&s.flag)) })?;
    f.entries.push(Entry { name: s.entry.clone(), subs: vec![sub], vars: Vec::new(), main: gate, pad });
    Ok(())
}

pub fn edit(f: &mut Flow, sales: &[Sale], label: &str) -> Res<()> {
    if sales.is_empty() {
        return Ok(());
    }
    base(f)?;
    for s in sales {
        add(f, s, label)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bfevfl::{Actor, Bf};

    fn ev(n: &str, kind: Kind) -> Event {
        Event { name: n.to_string(), kind }
    }

    fn sub(next: u16, entry: &str, items: Vec<(&str, Val)>) -> Kind {
        Kind::Sub { next, params: Some(Cont { items: items.into_iter().map(|(k, v)| (k.to_string(), v)).collect() }), flow: String::new(), entry: entry.to_string() }
    }

    fn s(x: &str) -> Val {
        Val::Str(x.to_string())
    }

    fn sample() -> Bf {
        let mut events: Vec<Event> = (0..400).map(|i| ev(&format!("Pad{i}"), Kind::Join { next: NONE })).collect();
        let greet = || vec![("Greeting_00", s("G0")), ("Greeting_01", s("G1")), ("Greeting02", s("G2"))];
        events[154] = ev("Event305", Kind::Action { next: NONE, actor: 1, act: 0, params: None });
        events[155] = ev("Event293", sub(NONE, ONE, vec![("ActorName", s("Obj_Heart_Piece")), ("Price", Val::Int(100)), ("SellerMessageId", s("M")), ("LookActorName", s("Enemy_Scrub_Business"))]));
        events[171] = ev("Event309", sub(NONE, POUCH, greet()));
        events[172] = ev("Event6", sub(NONE, POUCH, greet()));
        events[327] = ev("Event578", sub(NONE, POUCH, greet()));
        let actor = |n: &str, acts: &[&str], qs: &[&str]| Actor {
            name: n.to_string(),
            sub: String::new(),
            arg: String::new(),
            actions: acts.iter().map(|x| x.to_string()).collect(),
            queries: qs.iter().map(|x| x.to_string()).collect(),
            params: None,
            ep: 0xFFFF,
            cut: 1,
        };
        Bf {
            ver: [0, 4, 0, 0],
            bom: 0xFEFF,
            shift: 3,
            pad: 0,
            name: "T".to_string(),
            flows: vec![Flow {
                name: "T".to_string(),
                actors: vec![actor("PouchSystemActor", &[], &["X"]), actor(HOLDER, &[SET, "Other"], &[ASK, "Q"])],
                events,
                entries: vec![Entry { name: HEART.to_string(), subs: vec![327], vars: Vec::new(), main: 327, pad: 7 }],
            }],
            tls: Vec::new(),
        }
    }

    fn sale() -> Sale {
        Sale { entry: "OOTRR_Scrub_T".to_string(), flag: "HavePlayedEvent.X".to_string(), actor: "Item_Hookshot".to_string(), price: 30 }
    }

    #[test]
    fn the_base_moves_the_sale_values_into_the_three_rows_and_reads_them_back_as_arguments() {
        let mut b: Bf = sample();
        base(&mut b.flows[0]).unwrap();
        let back: Bf = Bf::parse(&b.write().unwrap()).unwrap();
        assert_eq!(back, b);
        let f: &Flow = &back.flows[0];
        let c: &Cont = f.events[155].kind.params().unwrap();
        assert_eq!(c.get("ActorName"), Some(&Val::Arg("SaleActor".to_string())));
        assert_eq!(c.get("Price"), Some(&Val::Arg("SalePrice".to_string())));
        assert_eq!(c.get("SellerMessageId"), Some(&Val::Arg("SaleMsg".to_string())));
        assert_eq!(c.get("LookActorName"), Some(&s("Enemy_Scrub_Business")));
        for i in [327, 172, 171] {
            let c: &Cont = f.events[i].kind.params().unwrap();
            assert_eq!(c.items.len(), 6, "{i}");
            assert_eq!(c.get("SaleActor"), Some(&s("Obj_Heart_Piece")), "{i}");
            assert_eq!(c.get("SalePrice"), Some(&Val::Int(100)), "{i}");
            assert_eq!(c.get("SaleMsg"), Some(&s("M")), "{i}");
        }
        assert_eq!(f.events.len(), 400);
        let mut again: Bf = back.clone();
        assert!(base(&mut again.flows[0]).is_err());
    }

    #[test]
    fn a_new_entry_asks_the_flag_calls_the_pouch_and_sets_the_flag_only_after_a_sale() {
        let mut b: Bf = sample();
        edit(&mut b.flows[0], &[sale()], "L").unwrap();
        let back: Bf = Bf::parse(&b.write().unwrap()).unwrap();
        assert_eq!(back, b);
        let f: &Flow = &back.flows[0];
        assert_eq!(f.events.len(), 404);
        let en: &Entry = f.entries.iter().find(|e| e.name == "OOTRR_Scrub_T").unwrap();
        assert_eq!((en.main, en.subs.clone(), en.pad), (403, vec![402], 7));
        let Kind::Switch { actor, query, cases, params } = &f.events[403].kind else { panic!("gate") };
        assert_eq!((*actor, *query), (1, 0));
        assert_eq!(cases, &vec![Case { val: 0, ev: 154 }, Case { val: 1, ev: 402 }]);
        assert_eq!(params.as_ref().unwrap().get("GameDataName"), Some(&s("HavePlayedEvent.X")));
        let Kind::Sub { next, params, flow, entry } = &f.events[402].kind else { panic!("call") };
        assert_eq!((*next, flow.as_str(), entry.as_str()), (401, "", POUCH));
        let p: &Cont = params.as_ref().unwrap();
        assert_eq!(p.get("SaleActor"), Some(&s("Item_Hookshot")));
        assert_eq!(p.get("SalePrice"), Some(&Val::Int(30)));
        assert_eq!(p.get("SaleMsg"), Some(&s("L")));
        assert_eq!(p.get("Greeting02"), Some(&s("G2")));
        let Kind::Switch { cases, params, .. } = &f.events[401].kind else { panic!("check") };
        assert_eq!(cases, &vec![Case { val: 0, ev: 400 }]);
        assert_eq!(params.as_ref().unwrap().get("GameDataName"), Some(&s(BOUGHT)));
        let Kind::Action { next, actor, act, params } = &f.events[400].kind else { panic!("set") };
        assert_eq!((*next, *actor, *act), (NONE, 1, 0));
        let p: &Cont = params.as_ref().unwrap();
        assert_eq!(p.get("Value"), Some(&Val::Bool(1)));
        assert_eq!(p.get("GameDataName"), Some(&s("HavePlayedEvent.X")));
        assert!(add(&mut b.flows[0], &sale(), "L").is_err());
    }

    #[test]
    fn no_sale_changes_nothing_and_a_different_file_is_refused() {
        let mut b: Bf = sample();
        edit(&mut b.flows[0], &[], "L").unwrap();
        assert_eq!(b, sample());
        for (i, k) in [(155, Kind::Join { next: NONE }), (327, Kind::Join { next: NONE }), (154, Kind::Join { next: NONE })] {
            let mut x: Bf = sample();
            x.flows[0].events[i].kind = k;
            assert!(edit(&mut x.flows[0], &[sale()], "L").is_err(), "{i}");
        }
        let mut x: Bf = sample();
        x.flows[0].events[172].name = "Other".to_string();
        assert!(base(&mut x.flows[0]).is_err());
        let mut x: Bf = sample();
        x.flows[0].entries.clear();
        assert!(add(&mut x.flows[0], &sale(), "L").is_err());
    }
}
