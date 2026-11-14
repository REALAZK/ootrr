use crate::bfevfl::{Bf, Cont, Event, Flow, Kind, Val};
use crate::error::{Error, Res};
use crate::qol::{edit_zs, Have};
use crate::romfs::Romfs;
use crate::staging::Out;

pub const FILE: &str = "Event/EventFlow/Npc_Hylia_048_BottleShop.bfevfl.zs";
pub const FLOW: &str = "Npc_Hylia_048_BottleShop";
pub const LOGIC_KEY: &str = "big_poe_count";
pub const COUNTER: &str = "GhostShop_CouponNum";
pub const QUERY: &str = "EventQueryIsGameDataInt";
pub const OLD: i32 = 400;
pub const NEW: i32 = 0;
pub const SITES: usize = 7;
pub const FINALE: i32 = 900;

pub const NOTE: &str = "The Ghost Shop owner gives the Bottle for the first big poe instead of the fifth. The point texts still talk about 500 points and the golden reward still needs ten big poes.";

fn compare(f: &Flow, e: &Event, at: i32) -> bool {
    let Kind::Switch { actor, query, params: Some(c), .. } = &e.kind else {
        return false;
    };
    let reads: bool = f.actors.get(*actor as usize).is_some_and(|a| a.name == "GameDataActor" && a.queries.get(*query as usize).is_some_and(|q| q == QUERY));
    reads && c.get("GameDataName") == Some(&Val::Str(COUNTER.to_string())) && c.get("Value") == Some(&Val::Int(at))
}

pub fn sites(f: &Flow, at: i32) -> Vec<usize> {
    (0..f.events.len()).filter(|&i| compare(f, &f.events[i], at)).collect()
}

fn set_value(c: &mut Cont, v: i32) {
    for (k, x) in c.items.iter_mut() {
        if k == "Value" {
            *x = Val::Int(v);
        }
    }
}

pub fn patch(raw: &[u8]) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    let f: &mut Flow = b.flows.iter_mut().find(|f| f.name == FLOW).ok_or(Error::Format("big poe one: the flow is not in the file"))?;
    let at: Vec<usize> = sites(f, OLD);
    if at.len() != SITES {
        return Err(Error::Msg(format!("big poe one: expected {SITES} point checks at {OLD}, found {}", at.len())));
    }
    if sites(f, FINALE).len() != 1 {
        return Err(Error::Format("big poe one: the check at 900 points is not there once"));
    }
    for i in at {
        if let Some(c) = f.events[i].kind.params_mut() {
            set_value(c, NEW);
        }
    }
    let out: Vec<u8> = b.write()?;
    let back: Bf = Bf::parse(&out)?;
    if back != b || back.flows.iter().any(|f| f.name == FLOW && !sites(f, OLD).is_empty()) {
        return Err(Error::Format("big poe one: the written flow does not read back"));
    }
    Ok(out)
}

pub fn stage(rf: &mut Romfs, have: Have) -> Res<Out> {
    edit_zs(rf, have, FILE, patch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bfevfl::{Actor, Case, Entry};

    fn dump() -> Option<std::path::PathBuf> {
        let p: std::path::PathBuf = std::env::var_os("OOTRR_ROMFS").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from("C:/o/nup dump/updates/131072/romfs"));
        if p.is_dir() {
            Some(p)
        } else {
            eprintln!("skipped: no romfs at {}, set OOTRR_ROMFS", p.display());
            None
        }
    }

    fn none(_p: &str) -> Option<Vec<u8>> {
        None
    }

    fn check(n: &str, at: i32) -> Event {
        Event {
            name: n.to_string(),
            kind: Kind::Switch {
                actor: 0,
                query: 0,
                cases: vec![Case { val: 0, ev: 0xFFFF }, Case { val: 1, ev: 0xFFFF }, Case { val: 2, ev: 0xFFFF }],
                params: Some(Cont {
                    items: vec![
                        ("GameDataName".to_string(), Val::Str(COUNTER.to_string())),
                        ("Index".to_string(), Val::Int(-1)),
                        ("Value".to_string(), Val::Int(at)),
                    ],
                }),
            },
        }
    }

    fn sample(n: usize) -> Bf {
        let mut events: Vec<Event> = (0..n).map(|i| check(&format!("E{i}"), OLD)).collect();
        events.push(check("Finale", FINALE));
        events.push(check("Other", 100));
        Bf {
            ver: [0, 4, 0, 0],
            bom: 0xFEFF,
            shift: 3,
            pad: 0,
            name: FLOW.to_string(),
            flows: vec![Flow {
                name: FLOW.to_string(),
                actors: vec![Actor {
                    name: "GameDataActor".to_string(),
                    sub: String::new(),
                    arg: String::new(),
                    actions: Vec::new(),
                    queries: vec![QUERY.to_string()],
                    params: None,
                    ep: 0xFFFF,
                    cut: 1,
                }],
                events,
                entries: vec![Entry { name: "Sell".to_string(), subs: Vec::new(), vars: Vec::new(), main: 0, pad: 0 }],
            }],
            tls: Vec::new(),
        }
    }

    fn values(b: &Bf) -> Vec<i32> {
        b.flows[0].events.iter().filter_map(|e| e.kind.params().and_then(|c| c.get("Value"))).filter_map(|v| if let Val::Int(n) = v { Some(*n) } else { None }).collect()
    }

    #[test]
    fn only_the_seven_checks_at_400_change() {
        let b: Bf = sample(SITES);
        let out: Vec<u8> = patch(&b.write().unwrap()).unwrap();
        let back: Bf = Bf::parse(&out).unwrap();
        let mut want: Vec<i32> = vec![NEW; SITES];
        want.extend([FINALE, 100]);
        assert_eq!(values(&back), want);
        assert!(patch(&out).is_err());
    }

    #[test]
    fn a_flow_with_the_wrong_number_of_checks_is_refused() {
        for n in [0, SITES - 1, SITES + 1] {
            assert!(patch(&sample(n).write().unwrap()).is_err(), "{n}");
        }
        let mut b: Bf = sample(SITES);
        b.flows[0].events.pop();
        b.flows[0].events.retain(|e| e.name != "Finale");
        assert!(patch(&b.write().unwrap()).is_err());
    }

    #[test]
    fn the_dump_flow_has_seven_checks_at_400_and_one_at_900() {
        let Some(root) = dump() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let raw: Vec<u8> = rf.read_zs(FILE).unwrap();
        let b: Bf = Bf::parse(&raw).unwrap();
        assert_eq!(b.write().unwrap(), raw);
        let f: &Flow = &b.flows[0];
        assert_eq!(sites(f, OLD), vec![242, 243, 244, 246, 248, 250, 252]);
        assert_eq!(sites(f, FINALE), vec![166]);
    }

    #[test]
    fn the_dump_patch_changes_exactly_those_events_and_keeps_the_size() {
        let Some(root) = dump() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let raw: Vec<u8> = rf.read_zs(FILE).unwrap();
        let out: Vec<u8> = patch(&raw).unwrap();
        assert_eq!(out.len(), raw.len());
        let (a, b): (Bf, Bf) = (Bf::parse(&raw).unwrap(), Bf::parse(&out).unwrap());
        let (x, y): (&Flow, &Flow) = (&a.flows[0], &b.flows[0]);
        assert_eq!(x.actors, y.actors);
        assert_eq!(x.entries, y.entries);
        assert_eq!(x.events.len(), y.events.len());
        let diff: Vec<usize> = (0..x.events.len()).filter(|&i| x.events[i] != y.events[i]).collect();
        assert_eq!(diff, vec![242, 243, 244, 246, 248, 250, 252]);
        for i in diff {
            assert_eq!(y.events[i].kind.params().and_then(|c| c.get("Value")), Some(&Val::Int(NEW)));
            assert_eq!(x.events[i].name, y.events[i].name);
        }
    }

    #[test]
    fn the_bottle_and_the_golden_reward_events_are_untouched() {
        let Some(root) = dump() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let raw: Vec<u8> = rf.read_zs(FILE).unwrap();
        let out: Vec<u8> = patch(&raw).unwrap();
        let (a, b): (Bf, Bf) = (Bf::parse(&raw).unwrap(), Bf::parse(&out).unwrap());
        for i in [319, 320, 231, 168, 16, 166, 234] {
            assert_eq!(a.flows[0].events[i], b.flows[0].events[i], "{i}");
        }
        let gives = |f: &Flow, i: usize| -> Option<String> {
            match f.events[i].kind.params()?.get("ActorName") {
                Some(Val::Str(s)) => Some(s.clone()),
                _ => None,
            }
        };
        assert_eq!(gives(&b.flows[0], 319).as_deref(), Some("Item_Bottle"));
        assert_eq!(gives(&b.flows[0], 320).as_deref(), Some("Obj_Rupee_Gold"));
    }

    #[test]
    fn staging_reads_an_earlier_output_when_there_is_one() {
        let Some(root) = dump() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let o: Out = stage(&mut rf, &none).unwrap();
        assert_eq!(o.path, FILE);
        assert_eq!(o.old, o.new);
        let first: Vec<u8> = o.z.clone();
        let again = |_p: &str| -> Option<Vec<u8>> { Some(first.clone()) };
        assert!(stage(&mut rf, &again).is_err());
    }
}
