use crate::bfevfl::{Event, Flow, Kind, Val};
use crate::error::{Error as E, Res};

pub fn clone_sub(f: &mut Flow, index: usize, name: &str, param: &str, old: &str, new: &str) -> Res<()> {
    let at: u16 = u16::try_from(f.events.len()).map_err(|_| E::Format("flow extra: index does not fit in 16 bits"))?;
    let me: u16 = u16::try_from(index).map_err(|_| E::Format("flow extra: index does not fit in 16 bits"))?;
    let e: &mut Event = f.events.get_mut(index).ok_or_else(|| E::Msg(format!("event index {index} is out of range")))?;
    if e.name != name {
        return Err(E::Msg(format!("event {index} is named {} and not {name}, the game files differ", e.name)));
    }
    let Kind::Sub { next, params, .. } = &mut e.kind else {
        return Err(E::Msg(format!("event {name} is not a sub flow call")));
    };
    let have: Option<&Val> = params.as_ref().and_then(|c| c.get(param));
    match have {
        Some(Val::Str(s)) if s == old => {}
        Some(Val::Str(s)) => return Err(E::Msg(format!("{param} of {name} holds {s} and not {old}, the game files differ"))),
        Some(_) => return Err(E::Msg(format!("{param} of {name} is not a name"))),
        None => return Err(E::Msg(format!("{name} has no parameter {param}"))),
    }
    let after: u16 = *next;
    *next = at;
    let mut c: Event = Event { name: format!("Extra{at}"), kind: e.kind.clone() };
    if let Kind::Sub { next, params, .. } = &mut c.kind {
        *next = after;
        if let Some(p) = params.as_mut() {
            for (k, v) in p.items.iter_mut() {
                if k == param {
                    *v = Val::Str(new.to_string());
                }
            }
        }
    }
    f.events.push(c);
    for en in f.entries.iter_mut() {
        if en.subs.contains(&me) {
            en.subs.push(at);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bfevfl::flow::NONE;
    use crate::bfevfl::{Actor, Bf, Cont, Entry};

    fn sub(n: &str, next: u16, item: &str) -> Event {
        Event {
            name: n.to_string(),
            kind: Kind::Sub {
                next,
                params: Some(Cont { items: vec![("Target".to_string(), Val::Int(3)), ("ReveiveItem".to_string(), Val::Str(item.to_string()))] }),
                flow: "NpcCommon".to_string(),
                entry: "NpcCommon_HandOver_NpcToPlayer_NoDisplay".to_string(),
            },
        }
    }

    fn sample() -> Bf {
        Bf {
            ver: [0, 4, 0, 0],
            bom: 0xFEFF,
            shift: 3,
            pad: 0,
            name: "T".to_string(),
            flows: vec![Flow {
                name: "T".to_string(),
                actors: vec![Actor {
                    name: "GameSystemActor".to_string(),
                    sub: String::new(),
                    arg: String::new(),
                    actions: Vec::new(),
                    queries: Vec::new(),
                    params: None,
                    ep: 0xFFFF,
                    cut: 1,
                }],
                events: vec![sub("Event0", 1, "Item_OddMushroom"), sub("Event1", NONE, "Item_Other")],
                entries: vec![
                    Entry { name: "Give".to_string(), subs: vec![0, 1], vars: Vec::new(), main: 0, pad: 0 },
                    Entry { name: "Retry".to_string(), subs: vec![1], vars: Vec::new(), main: 1, pad: 0 },
                ],
            }],
            tls: Vec::new(),
        }
    }

    fn round(b: &Bf) -> Bf {
        Bf::parse(&b.write().unwrap()).unwrap()
    }

    #[test]
    fn the_copy_runs_after_the_grant_and_joins_the_entries_that_list_it() {
        let mut b: Bf = sample();
        clone_sub(&mut b.flows[0], 0, "Event0", "ReveiveItem", "Item_OddMushroom", "Item_Longshot").unwrap();
        let back: Bf = round(&b);
        assert_eq!(back, b);
        let f: &Flow = &back.flows[0];
        assert_eq!(f.events.len(), 3);
        let Kind::Sub { next, params, .. } = &f.events[0].kind else { panic!("sub") };
        assert_eq!(*next, 2);
        assert_eq!(params.as_ref().and_then(|c| c.get("ReveiveItem")), Some(&Val::Str("Item_OddMushroom".to_string())));
        let Kind::Sub { next, params, flow, entry } = &f.events[2].kind else { panic!("sub") };
        assert_eq!(*next, 1);
        assert_eq!((flow.as_str(), entry.as_str()), ("NpcCommon", "NpcCommon_HandOver_NpcToPlayer_NoDisplay"));
        assert_eq!(params.as_ref().and_then(|c| c.get("ReveiveItem")), Some(&Val::Str("Item_Longshot".to_string())));
        assert_eq!(params.as_ref().and_then(|c| c.get("Target")), Some(&Val::Int(3)));
        assert_eq!(f.events[2].name, "Extra2");
        assert_eq!(f.entries[0].subs, vec![0, 1, 2]);
        assert_eq!(f.entries[1].subs, vec![1]);
    }

    #[test]
    fn a_wrong_name_kind_parameter_or_value_is_refused_and_nothing_changes() {
        for (i, n, p, old) in [
            (0, "Event9", "ReveiveItem", "Item_OddMushroom"),
            (5, "Event0", "ReveiveItem", "Item_OddMushroom"),
            (0, "Event0", "ActorName", "Item_OddMushroom"),
            (0, "Event0", "ReveiveItem", "Item_PocketEgg"),
            (0, "Event0", "Target", "Item_OddMushroom"),
        ] {
            let mut b: Bf = sample();
            assert!(clone_sub(&mut b.flows[0], i, n, p, old, "Item_Longshot").is_err(), "{i} {n} {p} {old}");
            assert_eq!(b, sample());
        }
        let mut b: Bf = sample();
        b.flows[0].events[0].kind = Kind::Join { next: 1 };
        assert!(clone_sub(&mut b.flows[0], 0, "Event0", "ReveiveItem", "Item_OddMushroom", "Item_Longshot").is_err());
    }

    #[test]
    fn a_tail_grant_keeps_its_end() {
        let mut b: Bf = sample();
        clone_sub(&mut b.flows[0], 1, "Event1", "ReveiveItem", "Item_Other", "Item_X").unwrap();
        let f: &Flow = &round(&b).flows[0];
        let Kind::Sub { next, .. } = &f.events[2].kind else { panic!("sub") };
        assert_eq!(*next, NONE);
        assert_eq!(f.entries[0].subs, vec![0, 1, 2]);
        assert_eq!(f.entries[1].subs, vec![1, 2]);
    }
}
