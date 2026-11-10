use crate::event_chests::walk::{Reach, Step, GENERIC_DEMOS};

#[derive(Default, Debug, Clone, PartialEq)]
pub struct Signals {
    pub chest_demo: bool,
    pub demos: Vec<String>,
    pub sets: Vec<String>,
    pub reads: Vec<String>,
    pub challenge: Vec<String>,
    pub dialog: Vec<String>,
    pub items: Vec<String>,
    pub equips: Vec<String>,
    pub songs: Vec<String>,
    pub calls: Vec<String>,
    pub guests: Vec<String>,
}

const SETS: [&str; 5] = [
    "GameDataActor.EventTriggerSetGameDataBool",
    "GameDataActor.EventTriggerSetGameDataInt",
    "GameDataActor.EventTriggerSetGameDataEnum",
    "GameDataActor.EventTriggerAddOrSubGameDataInt",
    "GameDataActor.EventTriggerSetDayTimeStruct",
];

fn arg<'a>(s: &'a Step, k: &str) -> Option<&'a str> {
    s.args.iter().find(|(a, _)| a == k).map(|(_, v)| v.as_str())
}

fn push(v: &mut Vec<String>, s: String) {
    if !v.contains(&s) {
        v.push(s);
    }
}

fn set_text(s: &Step) -> String {
    let n: &str = arg(s, "GameDataName").unwrap_or("?");
    let v: &str = arg(s, "Value").unwrap_or("");
    let op: &str = if s.head.ends_with("AddOrSubGameDataInt") { "+=" } else { "=" };
    if v.is_empty() { n.to_string() } else { format!("{n}{op}{v}") }
}

pub fn of(r: &Reach) -> Signals {
    let mut o: Signals = Signals {
        chest_demo: r.demos.iter().any(|d| GENERIC_DEMOS.contains(&d.as_str())) || r.stems.iter().any(|s| s.starts_with("Dm_SY_Treasurebox")),
        ..Signals::default()
    };
    for d in &r.demos {
        if !GENERIC_DEMOS.contains(&d.as_str()) {
            push(&mut o.demos, d.clone());
        }
    }
    for s in &r.steps {
        match s.kind {
            "action" | "clip" | "shot" => {
                if SETS.contains(&s.head.as_str()) {
                    push(&mut o.sets, set_text(s));
                }
                if s.head == "ChallengeSystemActor.EventExecuteProgressChallengeStep" {
                    let c: &str = arg(s, "ChallengeName").unwrap_or("?");
                    push(&mut o.challenge, format!("{c}:{}", arg(s, "StepName").unwrap_or("?")));
                }
                if s.head == "Npc_Navi.EventNaviRequestBallonMessage" {
                    push(&mut o.dialog, format!("navi:{}", arg(s, "Index").unwrap_or("?")));
                }
                if let Some(e) = arg(s, "EquipmentActorName") {
                    push(&mut o.equips, e.to_string());
                }
                if s.head.ends_with(".EventTriggerParticipateEvent") && !s.head.starts_with("Player_Event.") {
                    push(&mut o.guests, s.head.trim_end_matches(".EventTriggerParticipateEvent").to_string());
                }
            }
            "switch" => {
                if s.head.starts_with("GameDataActor.EventQuery")
                    && let Some(n) = arg(s, "GameDataName")
                {
                    push(&mut o.reads, n.to_string());
                }
            }
            "sub" | "subswitch" => {
                let from: &str = s.head.split(':').next().unwrap_or("");
                if from != s.stem {
                    push(&mut o.calls, s.head.clone());
                }
                if s.head.contains("Learning") {
                    push(&mut o.songs, s.head.clone());
                }
            }
            _ => {}
        }
    }
    for (role, item, _) in &r.items {
        push(&mut o.items, format!("{role}:{item}"));
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(kind: &'static str, head: &str, args: &[(&str, &str)]) -> Step {
        Step {
            stem: "F".to_string(),
            idx: 0,
            kind,
            head: head.to_string(),
            args: args.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(),
        }
    }

    #[test]
    fn reads_the_state_bearing_steps() {
        let mut r: Reach = Reach::default();
        r.demos = vec![GENERIC_DEMOS[0].to_string(), "Dm_X".to_string()];
        r.steps = vec![
            step("action", "GameDataActor.EventTriggerSetGameDataBool", &[("GameDataName", "A"), ("Value", "true")]),
            step("action", "GameDataActor.EventTriggerAddOrSubGameDataInt", &[("GameDataName", "B"), ("Value", "1")]),
            step("action", "ChallengeSystemActor.EventExecuteProgressChallengeStep", &[("ChallengeName", "C"), ("StepName", "S")]),
            step("action", "Npc_Navi.EventNaviRequestBallonMessage", &[("Index", "9")]),
            step("action", "Npc_Zora.EventTriggerParticipateEvent", &[]),
            step("switch", "GameDataActor.EventQueryIsGameDataBool", &[("GameDataName", "R")]),
            step("sub", "Other:LearningOcarina", &[]),
        ];
        r.items = vec![("give".to_string(), "Item_Z".to_string(), "v".to_string())];
        let s: Signals = of(&r);
        assert!(s.chest_demo);
        assert_eq!(s.demos, vec!["Dm_X".to_string()]);
        assert_eq!(s.sets, vec!["A=true".to_string(), "B+=1".to_string()]);
        assert_eq!(s.challenge, vec!["C:S".to_string()]);
        assert_eq!(s.dialog, vec!["navi:9".to_string()]);
        assert_eq!(s.guests, vec!["Npc_Zora".to_string()]);
        assert_eq!(s.reads, vec!["R".to_string()]);
        assert_eq!(s.songs, vec!["Other:LearningOcarina".to_string()]);
        assert_eq!(s.items, vec!["give:Item_Z".to_string()]);
    }

    #[test]
    fn same_stem_calls_are_not_listed() {
        let mut r: Reach = Reach::default();
        r.steps = vec![step("sub", "F:Local", &[]), step("sub", "G:Far", &[])];
        assert_eq!(of(&r).calls, vec!["G:Far".to_string()]);
    }
}


