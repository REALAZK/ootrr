use crate::bfevfl::{Bf, Cont, Event, Flow, Val};
use crate::error::{Error as E, Res};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Req {
    pub index: usize,
    pub event: String,
    pub param: String,
    pub old: String,
    pub new: String,
}

pub fn dict_id(z: &[u8]) -> Option<u32> {
    if z.get(..4)? != [0x28, 0xB5, 0x2F, 0xFD] {
        return None;
    }
    let d: u8 = *z.get(4)?;
    let n: usize = [0, 1, 2, 4][(d & 3) as usize];
    let mut id: u32 = 0;
    for i in 0..n {
        id |= (*z.get(5 + i)? as u32) << (8 * i);
    }
    if n == 0 { None } else { Some(id) }
}

pub fn set_param(b: &mut Bf, r: &Req) -> Res<()> {
    let f: &mut Flow = b.flows.first_mut().ok_or(E::Format("flow_locations: the file holds no flow"))?;
    let e: &mut Event = f
        .events
        .get_mut(r.index)
        .ok_or_else(|| E::Msg(format!("event index {} is out of range", r.index)))?;
    if e.name != r.event {
        return Err(E::Msg(format!("event {} is named {} and not {}, the game files differ", r.index, e.name, r.event)));
    }
    let c: &mut Cont = e
        .kind
        .params_mut()
        .ok_or_else(|| E::Msg(format!("event {} has no parameters", r.event)))?;
    for (k, v) in c.items.iter_mut() {
        if *k != r.param {
            continue;
        }
        return match v {
            Val::Str(s) | Val::Actor(s, _) if *s == r.old => {
                *s = r.new.clone();
                Ok(())
            }
            Val::Str(s) | Val::Actor(s, _) => {
                Err(E::Msg(format!("{} of {} holds {} and not {}, the game files differ", r.param, r.event, s, r.old)))
            }
            _ => Err(E::Msg(format!("{} of {} is not a name", r.param, r.event))),
        };
    }
    Err(E::Msg(format!("{} has no parameter {}", r.event, r.param)))
}

pub fn edit_bytes(raw: &[u8], rs: &[Req]) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    for r in rs {
        set_param(&mut b, r)?;
    }
    let out: Vec<u8> = b.write()?;
    if Bf::parse(&out)? != b {
        return Err(E::Format("flow_locations: the written flow does not parse back to the edited model"));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bfevfl::{Actor, Entry, Flow, Kind};
    use crate::zs;

    fn actor() -> Actor {
        Actor {
            name: "GetPouchContent".to_string(),
            sub: String::new(),
            arg: String::new(),
            actions: Vec::new(),
            queries: Vec::new(),
            params: None,
            ep: 0xFFFF,
            cut: 1,
        }
    }

    fn sub(n: &str, item: &str) -> Event {
        Event {
            name: n.to_string(),
            kind: Kind::Sub {
                next: 0xFFFF,
                params: Some(Cont {
                    items: vec![
                        ("ActorName".to_string(), Val::Str(item.to_string())),
                        ("Other".to_string(), Val::Str(item.to_string())),
                    ],
                }),
                flow: "GetPouchContent".to_string(),
                entry: "GetPouchContentByActorName".to_string(),
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
                actors: vec![actor()],
                events: vec![sub("Event0", "Item_A"), sub("Event1", "Item_A")],
                entries: vec![Entry { name: "Start".to_string(), subs: vec![0], vars: Vec::new(), main: 0, pad: 0 }],
            }],
            tls: Vec::new(),
        }
    }

    fn req(i: usize, ev: &str, old: &str, new: &str) -> Req {
        Req { index: i, event: ev.to_string(), param: "ActorName".to_string(), old: old.to_string(), new: new.to_string() }
    }

    #[test]
    fn edits_only_the_named_event_and_parameter() {
        let raw: Vec<u8> = sample().write().unwrap();
        let out: Vec<u8> = edit_bytes(&raw, &[req(1, "Event1", "Item_A", "Item_Longer_Name")]).unwrap();
        let b: Bf = Bf::parse(&out).unwrap();
        let get = |i: usize, k: &str| -> Option<Val> { b.flows[0].events[i].kind.params().and_then(|c| c.get(k).cloned()) };
        assert_eq!(get(1, "ActorName"), Some(Val::Str("Item_Longer_Name".to_string())));
        assert_eq!(get(1, "Other"), Some(Val::Str("Item_A".to_string())));
        assert_eq!(get(0, "ActorName"), Some(Val::Str("Item_A".to_string())));
    }

    #[test]
    fn a_changed_game_is_refused() {
        let raw: Vec<u8> = sample().write().unwrap();
        assert!(edit_bytes(&raw, &[req(0, "Event0", "Item_B", "Item_C")]).is_err());
        assert!(edit_bytes(&raw, &[req(0, "EventX", "Item_A", "Item_C")]).is_err());
        assert!(edit_bytes(&raw, &[req(9, "Event0", "Item_A", "Item_C")]).is_err());
    }

    #[test]
    fn editing_back_restores_the_original_bytes() {
        let raw: Vec<u8> = sample().write().unwrap();
        let out: Vec<u8> = edit_bytes(&raw, &[req(0, "Event0", "Item_A", "Item_Other")]).unwrap();
        let back: Vec<u8> = edit_bytes(&out, &[req(0, "Event0", "Item_Other", "Item_A")]).unwrap();
        assert_eq!(back, raw);
    }

    #[test]
    fn frame_header_dictionary_ids_are_read() {
        assert_eq!(dict_id(&zs::compress_raw(&[1, 2, 3], Some(1)).unwrap()), Some(1));
        assert_eq!(dict_id(&zs::compress_raw(&[1, 2, 3], Some(300)).unwrap()), Some(300));
        assert_eq!(dict_id(&zs::compress_raw(&[1, 2, 3], None).unwrap()), None);
        assert_eq!(dict_id(b"nope"), None);
    }
}
