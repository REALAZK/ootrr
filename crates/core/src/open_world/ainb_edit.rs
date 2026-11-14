use crate::ainb::Ainb;
use crate::error::{Error, Res};

pub const CHALLENGE: &str = "ChallengeName";
pub const STEP: &str = "ChallengeStepName";
const STR: usize = 3;
const KINDS: usize = 6;

pub fn path(challenge: &str) -> String {
    format!("Work/Challenge/{challenge}.game__challenge__Challenge.gyml")
}

pub fn check_of(a: &Ainb, i: usize) -> Option<(String, String)> {
    let c: &str = a.in_str(i, CHALLENGE)?;
    let s: &str = a.in_str(i, STEP)?;
    let name: &str = c.strip_prefix("Work/Challenge/")?.strip_suffix(".game__challenge__Challenge.gyml")?;
    Some((name.to_string(), s.to_string()))
}

pub fn feeds(a: &Ainb, i: usize, input: &str) -> bool {
    (0..KINDS).any(|k| (0..).map_while(|g| a.input(k, g)).any(|x| x.src >= 0 && x.src as usize == i && a.s(x.name) == input))
}

fn set_str(a: &mut Ainb, i: usize, name: &str, v: &str) -> Res<()> {
    let g: usize = a.find_in(i, STR, name).ok_or_else(|| Error::Msg(format!("open world: node {i} has no {name} input")))?;
    let o: u32 = a.pool.add(v);
    a.set_in_val(STR, g, &o.to_le_bytes())
}

pub fn retarget(a: &mut Ainb, from: (&str, &str), to: (&str, &str), consumer: Option<&str>) -> Res<usize> {
    let want: (String, String) = (from.0.to_string(), from.1.to_string());
    let hits: Vec<usize> = (0..a.params.len())
        .filter(|&i| check_of(a, i).as_ref() == Some(&want))
        .filter(|&i| consumer.is_none_or(|c| feeds(a, i, c)))
        .collect();
    let i: usize = match hits.as_slice() {
        [i] => *i,
        _ => return Err(Error::Msg(format!("open world: expected one {} {} step check, found {}", from.0, from.1, hits.len()))),
    };
    set_str(a, i, CHALLENGE, &path(to.0))?;
    set_str(a, i, STEP, to.1)?;
    Ok(i)
}

pub fn edit(raw: &[u8], jobs: &[((&str, &str), (&str, &str), Option<&str>)]) -> Res<Vec<u8>> {
    let mut a: Ainb = Ainb::parse(raw)?;
    for (from, to, consumer) in jobs {
        retarget(&mut a, *from, *to, *consumer)?;
    }
    let out: Vec<u8> = a.write()?;
    if Ainb::parse(&out)? != a {
        return Err(Error::Format("open world: the edited logic module does not parse back"));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::romfs::Romfs;
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

    fn checks(a: &Ainb) -> Vec<(usize, String, String)> {
        (0..a.params.len()).filter_map(|i| check_of(a, i).map(|(c, s)| (i, c, s))).collect()
    }

    #[test]
    fn a_path_round_trips_through_the_name() {
        assert_eq!(path("DoorOfTime"), "Work/Challenge/DoorOfTime.game__challenge__Challenge.gyml");
    }

    #[test]
    fn the_three_logic_modules_round_trip_and_only_the_chosen_check_moves() {
        let Some(root) = real() else {
            return;
        };
        let rf: Romfs = Romfs::open(&root).unwrap();
        for (file, job) in crate::open_world::LOGIC_JOBS {
            let raw: Vec<u8> = rf.read(file).unwrap();
            let a: Ainb = Ainb::parse(&raw).unwrap();
            assert_eq!(a.write().unwrap(), raw, "{file}");
            let before: Vec<(usize, String, String)> = checks(&a);
            let out: Vec<u8> = edit(&raw, &[job]).unwrap();
            let b: Ainb = Ainb::parse(&out).unwrap();
            let after: Vec<(usize, String, String)> = checks(&b);
            assert_eq!(before.len(), after.len(), "{file}");
            let moved: Vec<(&(usize, String, String), &(usize, String, String))> = before.iter().zip(after.iter()).filter(|(x, y)| x != y).collect();
            assert_eq!(moved.len(), 1, "{file}");
            let (x, y) = moved[0];
            assert_eq!((x.1.as_str(), x.2.as_str()), job.0, "{file}");
            assert_eq!((y.1.as_str(), y.2.as_str()), job.1, "{file}");
            assert_eq!(a.els, b.els, "{file}");
            assert!(out.len() >= raw.len());
            assert!(edit(&out, &[job]).is_err(), "{file}");
        }
    }

    #[test]
    fn the_fountain_edit_moves_the_mouth_areas_and_leaves_the_fish_and_the_step_trigger() {
        let Some(root) = real() else {
            return;
        };
        let rf: Romfs = Romfs::open(&root).unwrap();
        let raw: Vec<u8> = rf.read(crate::open_world::FOUNTAIN).unwrap();
        let a: Ainb = Ainb::parse(&raw).unwrap();
        let at: Vec<usize> = (0..a.params.len()).filter(|&i| check_of(&a, i) == Some(("Get_StoneOfWater".to_string(), "Step08".to_string()))).collect();
        assert_eq!(at.len(), 2);
        assert_eq!(at.iter().filter(|&&i| feeds(&a, i, "Logic_Create")).count(), 1);
        assert_eq!(at.iter().filter(|&&i| feeds(&a, i, "Logic_Delete")).count(), 1);
        let out: Vec<u8> = edit(&raw, &[crate::open_world::LOGIC_JOBS[0].1]).unwrap();
        let b: Ainb = Ainb::parse(&out).unwrap();
        let fish: usize = *at.iter().find(|&&i| feeds(&a, i, "Logic_Delete")).unwrap();
        let area: usize = *at.iter().find(|&&i| feeds(&a, i, "Logic_Create")).unwrap();
        assert_eq!(check_of(&b, fish), Some(("Get_StoneOfWater".to_string(), "Step08".to_string())));
        assert_eq!(check_of(&b, area), Some(("Get_StoneOfWater".to_string(), "Step06".to_string())));
        let set: Vec<usize> = (0..b.params.len()).filter(|&i| b.in_str(i, "StepName") == Some("Step08")).collect();
        assert_eq!(set.len(), 1);
    }
}
