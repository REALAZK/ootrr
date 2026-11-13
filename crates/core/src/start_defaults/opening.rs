use crate::bfevfl::{Bf, Event, Flow, Kind, Val};
use crate::error::{Error, Res};
use crate::flow_locations::dict_id;
use crate::intro_skip::{FILE, FLOW};
use crate::romfs::Romfs;
use crate::staging::{rewrite, Out};
use crate::zs;

pub const ACT: &str = "EventExecuteProgressChallengeStep";
pub const QUEST: &str = "Meet_DekuTree";
pub const FROM: &str = "Step01";
pub const OPEN_STEP: &str = "Step03";
pub const STEPS: [&str; 6] = ["NotReady", "Ready", "Step01", "Step02", "Step03", "Complete"];

fn text<'a>(e: &'a Event, k: &str) -> Option<&'a str> {
    match e.kind.params()?.get(k)? {
        Val::Str(s) => Some(s.as_str()),
        _ => None,
    }
}

fn is_step(f: &Flow, e: &Event) -> bool {
    let Kind::Action { actor, act, .. } = &e.kind else {
        return false;
    };
    let a: Option<&str> = f.actors.get(*actor as usize).and_then(|x| x.actions.get(*act as usize)).map(|s| s.as_str());
    a == Some(ACT) && text(e, "ChallengeName") == Some(QUEST)
}

pub fn patch(b: &mut Bf, step: &str) -> Res<usize> {
    if !STEPS.contains(&step) {
        return Err(Error::Msg(format!("opening: {QUEST} has no step {step}")));
    }
    let f: &mut Flow = b.flows.iter_mut().find(|x| x.name == FLOW).ok_or(Error::Format("opening: no OpeningEvent flow"))?;
    let hits: Vec<usize> = (0..f.events.len()).filter(|&i| is_step(f, &f.events[i])).collect();
    let i: usize = match hits.as_slice() {
        [i] => *i,
        _ => return Err(Error::Msg(format!("opening: expected one {QUEST} step request in {FLOW}, found {}", hits.len()))),
    };
    let now: Option<&str> = text(&f.events[i], "StepName");
    if now != Some(FROM) && now != Some(step) {
        return Err(Error::Msg(format!("opening: the {QUEST} request in {FLOW} asks for {}, not {FROM}", now.unwrap_or("nothing"))));
    }
    let slot: &mut Val = f.events[i]
        .kind
        .params_mut()
        .and_then(|c| c.items.iter_mut().find(|(k, _)| k == "StepName").map(|(_, v)| v))
        .ok_or(Error::Format("opening: the step request has no StepName"))?;
    *slot = Val::Str(step.to_string());
    Ok(i)
}

fn edit_raw(raw: &[u8], step: &str) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    patch(&mut b, step)?;
    let out: Vec<u8> = b.write()?;
    if Bf::parse(&out)? != b {
        return Err(Error::Format("opening: the patched flow does not parse back to the edited model"));
    }
    Ok(out)
}

pub fn stage(rf: &mut Romfs, outs: &mut Vec<Out>, step: &str) -> Res<()> {
    match outs.iter().position(|o| o.path == FILE) {
        Some(i) => {
            let id: Option<u32> = dict_id(&outs[i].z);
            let raw: Vec<u8> = rf.zs.decompress(&outs[i].z)?;
            let out: Vec<u8> = edit_raw(&raw, step)?;
            outs[i].new = out.len();
            outs[i].z = zs::compress_raw(&out, id)?;
        }
        None => outs.push(rewrite(rf, FILE, |raw| edit_raw(raw, step))?),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byml::{Byml, Node};
    use std::path::PathBuf as PB;

    fn real() -> Option<PB> {
        let p: PB = std::env::var("OOTRR_ROMFS").map(PB::from).unwrap_or_else(|_| PB::from("C:/o/nup dump/updates/131072/romfs"));
        if p.join("Pack").join("ZsDic.pack.zs").is_file() {
            Some(p)
        } else {
            eprintln!("skipped, no RomFS dump at {} (set OOTRR_ROMFS)", p.display());
            None
        }
    }

    fn steps_of(b: &Bf) -> Vec<String> {
        let f: &Flow = b.flows.iter().find(|x| x.name == FLOW).unwrap();
        f.events.iter().filter(|e| is_step(f, e)).filter_map(|e| text(e, "StepName").map(|s| s.to_string())).collect()
    }

    #[test]
    fn the_opening_request_moves_from_step01_to_step03_and_nothing_else_changes() {
        let Some(root) = real() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let raw: Vec<u8> = rf.read_zs(FILE).unwrap();
        let orig: Bf = Bf::parse(&raw).unwrap();
        assert_eq!(steps_of(&orig), vec![FROM.to_string()]);
        let mut b: Bf = orig.clone();
        let i: usize = patch(&mut b, OPEN_STEP).unwrap();
        assert_eq!(steps_of(&b), vec![OPEN_STEP.to_string()]);
        assert_eq!(patch(&mut b, OPEN_STEP).unwrap(), i);
        let f0: &Flow = orig.flows.iter().find(|x| x.name == FLOW).unwrap();
        let f1: &Flow = b.flows.iter().find(|x| x.name == FLOW).unwrap();
        assert_eq!(f0.events.len(), f1.events.len());
        let diff: Vec<usize> = (0..f0.events.len()).filter(|&j| f0.events[j] != f1.events[j]).collect();
        assert_eq!(diff, vec![i]);
        assert_eq!(f0.entries, f1.entries);
        assert_eq!(f0.actors, f1.actors);
        let out: Vec<u8> = edit_raw(&raw, OPEN_STEP).unwrap();
        assert_eq!(Bf::parse(&out).unwrap(), b);
        assert!(patch(&mut b, "Step09").is_err());
        let y: Byml = rf.read_byml(crate::chest_slot::LIST).unwrap();
        let q: &Node = y.root.get("Data").unwrap().get("Enum").unwrap().as_arr().unwrap().iter().find(|r| r.get("Hash") == Some(&Node::U32(crate::start_defaults::key("Step_Meet_DekuTree")))).unwrap();
        let names: Vec<&str> = q.get("RawValues").unwrap().as_arr().unwrap().iter().filter_map(|v| v.as_str()).collect();
        assert_eq!(names, STEPS.to_vec());
    }

    #[test]
    fn the_step_edit_lands_on_top_of_the_intro_skip_in_one_file() {
        let Some(root) = real() else {
            return;
        };
        let mut rf: Romfs = Romfs::open(&root).unwrap();
        let (mut outs, rep): (Vec<Out>, crate::intro_skip::Report) = crate::intro_skip::stage(&mut rf, crate::intro_skip::Opts::FULL).unwrap();
        let before: Vec<u8> = rf.zs.decompress(&outs.iter().find(|o| o.path == FILE).unwrap().z).unwrap();
        let n: usize = outs.len();
        stage(&mut rf, &mut outs, OPEN_STEP).unwrap();
        assert_eq!(outs.len(), n);
        let after: Bf = Bf::parse(&rf.zs.decompress(&outs.iter().find(|o| o.path == FILE).unwrap().z).unwrap()).unwrap();
        assert_eq!(steps_of(&after), vec![OPEN_STEP.to_string()]);
        let mut want: Bf = Bf::parse(&before).unwrap();
        patch(&mut want, OPEN_STEP).unwrap();
        assert_eq!(after, want);
        assert_eq!(rep.added_events, 2);
        let mut alone: Vec<Out> = Vec::new();
        stage(&mut rf, &mut alone, OPEN_STEP).unwrap();
        assert_eq!(alone.len(), 1);
        assert_eq!(steps_of(&Bf::parse(&rf.zs.decompress(&alone[0].z).unwrap()).unwrap()), vec![OPEN_STEP.to_string()]);
    }
}
