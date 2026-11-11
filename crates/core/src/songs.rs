use crate::bfevfl::{Bf, Cont, Event, Flow, Kind, Val};
use crate::error::{Error, Res};
use crate::romfs::Romfs;
use crate::staging::{rewrite, Out};

pub const FLOW: &str = "Event/EventFlow/Dm_SY_Ocarina.bfevfl.zs";
pub const KEEP: &str = "Item_Melody_ScarecrowSong";
pub const PREFIX: &str = "Item_Melody_";

fn text(v: &Val) -> Option<&str> {
    match v {
        Val::Str(s) | Val::Actor(s, _) => Some(s.as_str()),
        _ => None,
    }
}

fn zero(c: &mut Cont) -> bool {
    let hit: bool = c.get("Actor").and_then(text).is_some_and(|a| a.starts_with(PREFIX) && a != KEEP);
    if !hit {
        return false;
    }
    for (k, v) in c.items.iter_mut() {
        if k == "Num" && *v == Val::Int(1) {
            *v = Val::Int(0);
            return true;
        }
    }
    false
}

pub fn neutralize(raw: &[u8]) -> Res<(Vec<u8>, usize)> {
    let mut b: Bf = Bf::parse(raw)?;
    let f: &mut Flow = b.flows.first_mut().ok_or(Error::Format("songs: the flow holds no flow"))?;
    let mut n: usize = 0;
    for e in f.events.iter_mut() {
        let Event { kind, .. } = e;
        if let Kind::Action { params: Some(c), .. } = kind
            && zero(c)
        {
            n += 1;
        }
    }
    if n == 0 {
        return Err(Error::Format("songs: no song grant found in the shared lesson flow, the game files differ"));
    }
    let out: Vec<u8> = b.write()?;
    if Bf::parse(&out)? != b {
        return Err(Error::Format("songs: the written flow does not parse back to the edited model"));
    }
    Ok((out, n))
}

pub fn stage(rf: &mut Romfs) -> Res<Out> {
    rewrite(rf, FLOW, |raw| neutralize(raw).map(|x| x.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bfevfl::{Actor, Entry};

    fn grant(name: &str, item: &str, num: i32) -> Event {
        Event {
            name: name.to_string(),
            kind: Kind::Action {
                next: 0xFFFF,
                actor: 0,
                act: 0,
                params: Some(Cont {
                    items: vec![
                        ("Actor".to_string(), Val::Str(item.to_string())),
                        ("Num".to_string(), Val::Int(num)),
                        ("IsGetEvent".to_string(), Val::Bool(0)),
                    ],
                }),
            },
        }
    }

    fn sample() -> Bf {
        Bf {
            ver: [0, 4, 0, 0],
            bom: 0xFEFF,
            shift: 3,
            pad: 0,
            name: "Test".to_string(),
            flows: vec![Flow {
                name: "Test".to_string(),
                actors: vec![Actor {
                    name: "Pouch".to_string(),
                    sub: String::new(),
                    arg: String::new(),
                    actions: vec!["Add".to_string()],
                    queries: Vec::new(),
                    params: None,
                    ep: 0xFFFF,
                    cut: 1,
                }],
                events: vec![
                    grant("E0", "Item_Melody_SariaSong", 1),
                    grant("E1", "Item_Melody_ScarecrowSong", 1),
                    grant("E2", "Item_Hookshot", 1),
                ],
                entries: vec![Entry { name: "Start".to_string(), subs: vec![0], vars: Vec::new(), main: 0, pad: 0 }],
            }],
            tls: Vec::new(),
        }
    }

    #[test]
    fn song_grants_drop_to_zero_but_the_scarecrow_and_other_items_keep_theirs() {
        let raw: Vec<u8> = sample().write().unwrap();
        let (out, n) = neutralize(&raw).unwrap();
        assert_eq!(n, 1);
        let b: Bf = Bf::parse(&out).unwrap();
        let num = |i: usize| b.flows[0].events[i].kind.params().and_then(|c| c.get("Num").cloned());
        assert_eq!(num(0), Some(Val::Int(0)));
        assert_eq!(num(1), Some(Val::Int(1)));
        assert_eq!(num(2), Some(Val::Int(1)));
        assert!(neutralize(&out).is_err());
    }
}
