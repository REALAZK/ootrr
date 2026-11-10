use crate::event_chests::signals::Signals;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Verdict {
    StripSafe,
    StripButSetFlags,
    KeepEvent,
    KeepVanilla,
    FlowEdit,
}

impl Verdict {
    pub fn name(&self) -> &'static str {
        match self {
            Verdict::StripSafe => "strip_safe",
            Verdict::StripButSetFlags => "strip_but_set_flags",
            Verdict::KeepEvent => "keep_event",
            Verdict::KeepVanilla => "keep_vanilla",
            Verdict::FlowEdit => "flow_edit",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cons {
    pub token: String,
    pub file: String,
    pub path: String,
    pub class: &'static str,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Judged {
    pub verdict: Verdict,
    pub conf: &'static str,
    pub why: String,
    pub lost: Vec<String>,
    pub stale: Vec<String>,
    pub needs: Vec<String>,
}

pub const VERIFIED: [&str; 1] = ["Get_StoneOfForest_GetSword"];

const REVIEWED: [(&str, &str, &str, &str); 27] = [
    ("KokiriForest_TreasureBox_NaviChatOff", "Pack/NaviBallonMess", "cosmetic", "Navi balloon message condition"),
    ("InsideJabuJabuBelly_Room12_TBoxDemo", "InsideJabuJabuBelly_Level_Room_12", "cosmetic", "Ruto sensor area is deleted once the demo ran"),
    ("InsideJabuJabuBelly_Room09_TBoxDemo", "InsideJabuJabuBelly_Level_Room_09", "cosmetic", "Ruto sensor area is deleted once the demo ran"),
    ("InsideJabuJabuBelly_Room09_TBoxDemo", "InsideJabuJabuBelly_Dynamic", "gating", "an NPC air wall is present only while the flag is false"),
    ("SubEvent_GetIceArrow_IsCompleted_Exp", "Npc_Gerudo_004", "cosmetic", "gatekeeper balloon and dialogue branches only"),
    ("NaviRepeat_Minigame_DampeRace_End", "MiniGame_DampeRace", "cosmetic", "Navi repeat hint flag inside the race flow"),
    ("Market_MiniGame_ChoiceTreasure_NumOfHit", "GetPouchContent", "cosmetic", "picks the get message variant"),
    ("HavePlayedEvent.NaviTips_Gauntlet_01", "GerudoTrainingGround_Level_Room_03", "cosmetic", "Navi tip trigger"),
    ("HavePlayedEvent.NaviTips_Gauntlet_03", "GerudoTrainingGround_Level_Room_03", "cosmetic", "Navi tip trigger"),
    ("UI_MapMenuIsUnvisibleByLsPush", "Sequence/Event.module", "cosmetic", "map menu hide toggle during the minigame"),
    ("MiniGame_SmallKeyNum", "MiniGame_ChainLockedDoor", "gating", "chain lock door counts the key flag"),
    ("MiniGame_SmallKeyNum", "DoorChainLock", "gating", "chain lock door actor pack"),
    ("MiniGame_ChoiceTreasure_OpenedEitherBox", "Obj_HrlCity_Shop_TBox", "gating", "the minigame chest actor reads it"),
    ("MiniGame_ChoiceTreasure_GimickRegeneration", "MiniGame_SequentialChoiceTreasure", "gating", "minigame room logic regenerates the chests"),
    ("MiniGame_ChoiceTreasure_IsClear", "MiniGame_SequentialChoiceTreasure", "gating", "minigame room logic"),
    ("MiniGame_ChoiceTreasure_IsClear", "Market_Child_Static", "gating", "heart piece anchor reads the clear flag"),
    ("HavePlayedEvent.MiniGame_DampeRace_OpenGoalTBox", "UnderTheGrave", "gating", "grave logic reads that the goal chest was opened"),
    ("Minigame_DampeRace_TreasureBoxState", "UnderTheGrave", "gating", "grave logic picks which goal chest appears"),
    ("HavePlayedEvent.Dm_DesertColossus_0030", "Dm_SY_Ocarina", "gating", "ocarina flow branches on the demo"),
    ("HavePlayedEvent.Dm_DesertColossus_0030", "Market_Child_Prefab", "gating", "Market logic module reads the demo flag"),
    ("AllSage_MeetSheikCount", "Dm_MelodyGet_", "gating", "the other Sheik song demos branch on the counter"),
    ("SageOfSpirit_IsAfter_Step07_Exp", "Npc_Gerudo", "gating", "quest step read by Gerudo flows"),
    ("SageOfWater_GetWaterSerenade", "SageOfWater", "cosmetic", "same story flow"),
    ("HavePlayedEvent.NaviHint_Shield_Hylian", "AllSage", "cosmetic", "Navi hint tracking"),
    ("HavePlayedEvent.NaviHint_Shield_Hylian", "ResidentEventList", "cosmetic", "resident event listing"),
    ("NaviHint_Shield_Hylian_PlayedEventFlow", "Sequence/", "cosmetic", "Navi hint tracking read by the generic event sequence"),
    ("NaviChat_Serious_Time", "Npc_Navi", "cosmetic", "Navi chat timer"),
];
pub fn classify(token: &str, file: &str, path: &str) -> (&'static str, String) {
    if path.ends_with("GetEventName") {
        return ("self", String::new());
    }
    for (t, f, c, n) in REVIEWED {
        if token == t && file.contains(f) {
            return (c, n.to_string());
        }
    }
    ("gating", "consumer not reviewed".to_string())
}

pub fn flag_of(set: &str) -> &str {
    set.split(['=', '+']).next().unwrap_or(set)
}

fn uniq(v: &mut Vec<String>, s: String) {
    if !v.contains(&s) {
        v.push(s);
    }
}

pub fn judge(event: &str, flow: &str, s: &Signals, cons: &[Cons]) -> Judged {
    let gate: Vec<&Cons> = cons.iter().filter(|c| c.class == "gating").collect();
    let mut lost: Vec<String> = Vec::new();
    for x in &s.dialog {
        uniq(&mut lost, x.clone());
    }
    for x in &s.demos {
        uniq(&mut lost, format!("demo:{x}"));
    }
    for x in &s.equips {
        uniq(&mut lost, format!("equip:{x}"));
    }
    for x in &s.challenge {
        uniq(&mut lost, format!("challenge:{x}"));
    }
    for x in &s.sets {
        uniq(&mut lost, format!("flag:{x}"));
    }
    for x in &s.guests {
        uniq(&mut lost, format!("guest:{x}"));
    }
    let base: String = format!(
        "flow {flow} {} and sets {}",
        if s.chest_demo { "plays the generic chest demo" } else { "has no chest demo" },
        if s.sets.is_empty() { "no flags".to_string() } else { s.sets.join(", ") }
    );
    if !gate.is_empty() || !s.songs.is_empty() {
        let mut needs: Vec<String> = Vec::new();
        for c in &gate {
            uniq(&mut needs, format!("{} read by {} ({})", c.token, c.file, c.note));
        }
        for x in &s.songs {
            uniq(&mut needs, format!("song {x}"));
        }
        let own: bool = !s.equips.is_empty();
        return Judged {
            verdict: if own { Verdict::StripButSetFlags } else { Verdict::KeepEvent },
            conf: "guess",
            why: format!("{base}, and its state is read elsewhere, so the event must still run or its flags must be set another way"),
            lost: if own { s.dialog.iter().chain(s.equips.iter()).cloned().collect() } else { Vec::new() },
            stale: if own { Vec::new() } else { s.dialog.clone() },
            needs,
        };
    }
    let conf: &'static str = if VERIFIED.contains(&event) {
        "verified"
    } else if s.guests.is_empty() {
        "flow_evidence"
    } else {
        "guess"
    };
    Judged {
        verdict: Verdict::StripSafe,
        conf,
        why: format!("{base}, and nothing outside the event reads its state"),
        lost,
        stale: Vec::new(),
        needs: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cons(class: &'static str) -> Cons {
        Cons { token: "T".to_string(), file: "F".to_string(), path: String::new(), class, note: "n".to_string() }
    }

    #[test]
    fn plain_wrapper_is_strip_safe() {
        let s: Signals = Signals { chest_demo: true, dialog: vec!["navi:1".to_string()], ..Signals::default() };
        let j: Judged = judge("E", "F:G", &s, &[]);
        assert_eq!(j.verdict, Verdict::StripSafe);
        assert_eq!(j.conf, "flow_evidence");
        assert_eq!(j.lost, vec!["navi:1".to_string()]);
    }

    #[test]
    fn a_gating_reader_keeps_the_event() {
        let s: Signals = Signals { chest_demo: true, sets: vec!["A=true".to_string()], ..Signals::default() };
        let j: Judged = judge("E", "F:G", &s, &[cons("gating")]);
        assert_eq!(j.verdict, Verdict::KeepEvent);
        assert!(j.needs[0].contains("read by F"));
    }

    #[test]
    fn cosmetic_readers_and_guests_lower_confidence_only() {
        let s: Signals = Signals { guests: vec!["Npc".to_string()], ..Signals::default() };
        let j: Judged = judge("E", "F:G", &s, &[cons("cosmetic")]);
        assert_eq!((j.verdict, j.conf), (Verdict::StripSafe, "guess"));
    }

    #[test]
    fn songs_keep_the_event() {
        let s: Signals = Signals { songs: vec!["X:Learn".to_string()], ..Signals::default() };
        assert_eq!(judge("E", "F:G", &s, &[]).verdict, Verdict::KeepEvent);
    }

    #[test]
    fn verified_events_say_so() {
        let j: Judged = judge("Get_StoneOfForest_GetSword", "F:G", &Signals::default(), &[]);
        assert_eq!(j.conf, "verified");
    }

    #[test]
    fn classify_uses_the_review_table() {
        assert_eq!(classify("SubEvent_GetIceArrow_IsCompleted_Exp", "Event/EventFlow/Npc_Gerudo_004.bfevfl.zs", "").0, "cosmetic");
        assert_eq!(classify("InsideJabuJabuBelly_Room09_TBoxDemo", "Banc/InsideJabuJabuBelly/InsideJabuJabuBelly_Dynamic.x", "A.Presence.FlagName").0, "gating");
        assert_eq!(classify("X", "Logic/a", "").0, "gating");
        assert_eq!(classify("X", "Banc/a", "Actors.Dynamic.GetEventName").0, "self");
        assert_eq!(flag_of("A+=1"), "A");
        assert_eq!(flag_of("A=true"), "A");
    }
}





