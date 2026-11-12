use std::collections::BTreeMap as Map;

use crate::bfevfl::{Bf, Flow, Kind};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class {
    Control,
    DataSet,
    Grant,
    Bottle,
    Challenge,
    Leave,
    Talk,
    TimeSet,
    NpcFlag,
    WorldBool,
    Inert,
    Gameplay,
    QData,
    QPouch,
    QBottle,
    QChoice,
    QRandom,
    QTime,
    QNpc,
    QPlayer,
    QScene,
    QEngine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fit {
    Exact,
    Stub,
    Inert,
    Opaque,
}

impl Class {
    pub fn fit(self) -> Fit {
        match self {
            Class::Control | Class::DataSet | Class::Grant | Class::Challenge | Class::QData | Class::QPouch => Fit::Exact,
            Class::Bottle
            | Class::QBottle
            | Class::Leave
            | Class::TimeSet
            | Class::NpcFlag
            | Class::WorldBool
            | Class::QChoice
            | Class::QRandom
            | Class::QTime
            | Class::QNpc
            | Class::QPlayer => Fit::Stub,
            Class::Talk | Class::Inert => Fit::Inert,
            Class::Gameplay | Class::QScene | Class::QEngine => Fit::Opaque,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Class::Control => "flow control (sub, fork, join, label)",
            Class::DataSet => "GameData writes",
            Class::Grant => "item and rupee grants",
            Class::Bottle => "bottle slots and shop stock",
            Class::Challenge => "challenge step requests",
            Class::Leave => "scene change, transition, save, death",
            Class::Talk => "messages and balloons",
            Class::TimeSet => "time of day writes",
            Class::NpcFlag => "per NPC talk flags",
            Class::WorldBool => "event bool to an actor",
            Class::Inert => "presentation (camera, sound, anim, move, fx)",
            Class::Gameplay => "gameplay engine (player, minigame, enemy, ocarina)",
            Class::QData => "GameData reads",
            Class::QPouch => "pouch and rupee reads",
            Class::QBottle => "bottle slot and wallet room reads",
            Class::QChoice => "player message choice",
            Class::QRandom => "random choice",
            Class::QTime => "time of day reads",
            Class::QNpc => "per NPC talk flag reads",
            Class::QPlayer => "player equipment and state reads",
            Class::QScene => "spatial, AI, camera reads",
            Class::QEngine => "other engine reads",
        }
    }
}

const DATA_SET: &[&str] = &[
    "EventTriggerSetGameDataBool",
    "EventTriggerSetGameDataInt",
    "EventTriggerSetGameDataEnum",
    "EventTriggerSetGameDataFloat",
    "EventTriggerSetGameDataString64",
    "EventTriggerSetGameDataBoolArrayAllOnce",
    "EventTriggerAddOrSubGameDataInt",
    "EventTriggerAddOrSubGameDataFloat",
    "EventTriggerSubGameDataInt",
    "EventTriggerCopyGameDataInt",
    "EventTriggerCopyGameDataFloat",
    "EventTriggerCopyGameDataString64",
    "EventTriggerSetHavePlayedGetEvent",
    "EventTriggerSetHavePlayedGetEventAtShopFlag",
    "EventTriggerResetHavePlayedGetEventAtShopFlag",
    "EventTriggerSetScarecrowMelodyGameData",
    "EventTriggerSetPierreMelodyGameData",
    "EventGameDataSetIsBlewOcarinaToGossipStone",
    "EventScrubBusinessWriteGameDataHaveTalked",
    "EventScrubBusinessWriteGameDataBancItemName",
    "EventScrubBusinessWriteGameDataLeastItemName",
    "EventTriggerSetWasAppearInEvent",
    "EventTriggerSetHaveTalked",
];

const GRANT: &[&str] = &[
    "EventTriggerAddOrRemovePouchContent",
    "EventTriggerIncreaseRupee",
    "EventTriggerIncreaseRupeeBySell",
    "EventTriggerAddGameDataIntToRupee",
    "EventGetDungeonMap",
    "EventGetDungeonCompass",
    "EventGetDungeonBossKey",
    "EventGetDungeonSmallKey",
    "EventTriggerAddOwnToPouch",
    "EventTriggerRemoveOwnedMask",
    "EventPorterSendRupee",
];

const BOTTLE: &[&str] = &[
    "EventTriggerPushToEmptyBottle",
    "EventTriggerRemoveFromSelectedBottle",
    "EventTriggerRemoveFromSpecificBottle",
    "EventTriggerConsumeDrinkFromSelectedBottle",
    "EventTriggerRemoveShopSelectedItemFromBottle",
    "EventTriggerSetSelectBottleInfoToShop",
    "EventTriggerGetItemAtShop",
    "EventTriggerGetBottleItemAtShop",
    "EventTriggerShopItemPutInPouch",
    "EventTriggerAddShopItemSoldNum",
    "EventTriggerPayItemPriceAtShop",
    "EventTriggerBorrowMaskAtShop",
    "EventTriggertExchangeEnemyMuseumPouchAndState",
    "EventTriggertExchangeEnemyMuseumPouchAndStateForContinuous",
    "EventTriggertRestorePouchFromEnemyMuseum",
    "EventTriggerRemoveSwordBigGoronChallengeActor",
    "EventTriggerRequestDeletePlayerEquipmentFromPouch",
    "EventTriggerRemoveWeaponKeepSheath",
];

const CHALLENGE: &[&str] = &[
    "EventExecuteProgressChallengeStep",
    "EventTriggerProgressChallengeStep",
    "EventExecuteRollbackChallengeStep",
];

const LEAVE: &[&str] = &[
    "EventTransition",
    "EventSuspend",
    "EventRequestQuit",
    "EventTriggerRequestAutoSave",
    "EventSaveAndWaitTemporarySave",
    "EventManualRequestAutoSaveCaption",
    "EventTriggerRequestDeath",
    "EventPlayerDie",
    "EventTriggerDeathSignal",
    "EventOpenGameOverScreen",
    "EventChangeDungeonRoom",
    "EventPlayerWarpToDungeonEntrance",
];

const TIME_SET: &[&str] = &[
    "EventTrigger_SetTime",
    "EventTriggerSetDayTimeStruct",
    "EventExecuteSetTimeStruct",
    "Event_SetTimeForFlowchart",
    "EventTrigger_RequestVisualOnlyTime",
    "EventTriggerSetIsTimeElapsedToNightByOcarinaSunSong",
];

const NPC_FLAG: &[&str] = &[
    "EventTriggerSetRecentTalkFlag",
    "EventTriggerForceSetRecentTalkFlag",
    "EventDisableRecentTalkFlag",
    "EventTriggerSetMaskTalkFlag",
    "EventTriggerForceSetMaskTalkFlag",
    "EventTriggerForceSetAmiiboMaskTalkFlag",
    "EventTriggerSetVolatileGeneralPurposeFlag",
    "EventAIScheduleNPCForceResetMaskTalkFlag",
];

const TALK: &[&str] = &[
    "EventTalk",
    "EventTalkWithInstanceNameLabel",
    "EventTalkIgnoreSameTalkAS",
    "EventTalkWithSpeakerName",
    "EventTalkSariaSongPlace",
    "EventPlayerTalk",
    "EventPoeSpiritTalk",
    "EventTriggerBalloonMessage",
    "EventNaviRequestBallonMessage",
    "EventNaviRequestBalloon",
    "EventMiniGameDiggingHoleEntryBalloonMessage",
    "EventTriggerMiniGameDiggingHoleBalloonMessage",
];

const INERT: &[&str] = &[
    "Camera",
    "DoF",
    "Palette",
    "Weather",
    "Wind",
    "Foliage",
    "Lod",
    "Sound",
    "Bgm",
    "Voice",
    "XLink",
    "ELink",
    "Effect",
    "Rumble",
    "Fade",
    "LookAt",
    "Look",
    "Turn",
    "PlayAS",
    "ASWait",
    "Emotion",
    "Emote",
    "Speaker",
    "Participate",
    "Offstage",
    "Navi",
    "Warp",
    "Move",
    "Door",
    "ShowItem",
    "Display",
    "Wait",
    "Ducking",
    "Skip",
    "Cull",
    "Shadow",
    "Reverb",
    "Reflection",
    "Caustics",
    "Pixel",
    "Resolution",
    "Locational",
    "Terrain",
    "Cloth",
    "Model",
    "Anm",
    "Message",
    "Dialog",
    "Screen",
    "UI",
    "Timer",
    "ActionGuide",
    "Bust",
    "Cut",
    "Pause",
    "Ragdoll",
    "FootIK",
    "Occlusion",
    "Snapshot",
    "Staff",
    "Loading",
    "Preload",
    "Beforehand",
    "Attention",
    "Gauge",
    "Heart",
    "Draw",
    "Hide",
    "Unshow",
    "Show",
    "Open",
    "Close",
    "Nothing",
    "SendErrorToEventResource",
    "Bind",
    "Delete",
    "Emit",
    "Stop",
    "Start",
    "Reset",
    "Set",
    "Create",
    "Appear",
    "Mask",
    "Greeting",
    "Conversate",
    "Backflip",
    "Dance",
    "Sleep",
    "Wake",
    "Stand",
    "Kneel",
    "Fall",
    "Jump",
    "Swell",
    "Rise",
];

const KEEP_OPAQUE: &[&str] = &[
    "Minigame",
    "MiniGame",
    "Frog",
    "Fishing",
    "Horse",
    "Cucco",
    "Heal",
    "Born",
    "Fairy",
    "Equip",
    "Unequip",
    "Ocarina",
    "Bottle",
    "Enemy",
    "Death",
    "Scrub",
    "TBox",
    "TreasureBox",
    "Poe",
    "Diving",
    "Digging",
    "Goron",
    "Wallmaster",
    "Boss",
    "Gossip",
    "Scarecrow",
    "Pierre",
    "Bonooru",
    "Farore",
    "Dream",
    "AISchedule",
    "Kokiri",
    "Stray",
    "Bomb",
    "SmallFarm",
    "Item",
    "Pouch",
    "Rupee",
    "Weapon",
    "Shield",
    "Sword",
    "Arrow",
    "Seed",
    "Boots",
    "Tune",
    "Melody",
    "Play",
    "Sheath",
    "Drawn",
    "PutAway",
    "Dungeon",
    "Event_Rebuild",
    "Phantom",
    "Shell",
    "Dark",
    "Tektite",
    "Middle",
    "Super",
    "Tombstone",
    "Chemical",
    "Pillar",
    "Training",
    "Warp",
];

const SOFT_INERT: &[&str] = &["Warp", "Wait", "Move", "Heart", "Draw", "Stop", "Start", "Set", "Create", "Delete", "Reset", "Open", "Close", "Show", "Play", "Mask", "Appear", "Item", "Bottle", "Fall", "Jump"];

const HIDDEN_WRITER: &[&str] = &[
    "EventAIScheduleNPCForceWarpToCurrentAnchor",
    "EventCloseShopBuyScreen",
    "EventOpenShopBuyScreen",
    "EventTriggerSetForGetWindow",
    "EventTriggerSetOwnNameToMessage",
    "EventWaitGameOverButton",
    "EventWaitGameOverReturnTitleCheckButton",
    "EventAIScheduleNPCForceSecurityLifted",
    "EventAIScheduleReceiveAndWearMask",
    "EventTriggerGetFirstExpansionMaxNumActor",
    "EventTriggerInitializeBuyShop",
    "EventTriggerSetBottleForGetWindow",
    "EventTriggerTreasureBoxOpened",
];

const INERT_EXACT: &[&str] = &[
    "EventDemoProductSupportComment",
    "EventOverwriteSceneShadingParams",
    "EventTriggerResetPlayedNoteAppliedPitch",
    "EventTriggerSetOwnAsInstrumentPlayer",
    "EventOcarinaResultCamera",
    "EventTriggerShortFadeOcarinaMelodyHandle",
    "EventTriggerKokiriRequestComeCloserFairy",
    "EventPlayerSlowWalkStop",
    "EventPlayerStopInAir",
    "EventPlayerContinueWalk",
    "EventPlayerConversate",
    "EventPlayerSetExtraOrderDecelerateToStop",
    "EventPlayerPutAway",
    "EventPlayerOffStopMode",
    "EventPlayerOnStopMode",
    "EventPlayerOnMotionTypeStatic",
    "EventPlayerOffMotionTypeStatic",
    "EventRemoveProjectiles",
    "EventTriggerRequestRideMe",
    "EventTriggerReplaceTextureDoReplace",
    "EventTriggerWmThunderOneShot",
    "EventTriggerWorldMgrThunderMgrChangePatternKey",
    "EventEnterGameSafetySection",
    "EventTriggerLeaveGameSafetySection",
    "EventRefindRangeGameSafetySection",
];

const INERT_PREFIX: &[&str] = &[
    "EventTriggerPlayerEventStart",
    "EventPlayerBackMove",
    "EventPlayerMove",
    "EventCreateShowItem",
    "EventDeleteShowItem",
    "EventIgnitionShowItem",
    "EventTriggerExcludeNextTalkChoice",
];

pub fn action(n: &str) -> Class {
    if DATA_SET.contains(&n) {
        return Class::DataSet;
    }
    if GRANT.contains(&n) {
        return Class::Grant;
    }
    if BOTTLE.contains(&n) {
        return Class::Bottle;
    }
    if CHALLENGE.contains(&n) {
        return Class::Challenge;
    }
    if LEAVE.contains(&n) || n.starts_with("EventReserveBancChange") {
        return Class::Leave;
    }
    if TIME_SET.contains(&n) {
        return Class::TimeSet;
    }
    if NPC_FLAG.contains(&n) {
        return Class::NpcFlag;
    }
    if TALK.contains(&n) {
        return Class::Talk;
    }
    if n.starts_with("EventTriggerSetEventBool") || n == "EventSetEventResult" {
        return Class::WorldBool;
    }
    if HIDDEN_WRITER.contains(&n) {
        return Class::Gameplay;
    }
    if INERT_EXACT.contains(&n) || INERT_PREFIX.iter().any(|p| n.starts_with(p)) {
        return Class::Inert;
    }
    let inert: bool = INERT.iter().any(|p| n.contains(p));
    let hard: bool = KEEP_OPAQUE.iter().any(|p| n.contains(p));
    let soft: bool = SOFT_INERT.iter().any(|p| n.contains(p));
    if inert && !hard {
        return Class::Inert;
    }
    if inert && hard && soft && presentation_only(n) {
        return Class::Inert;
    }
    Class::Gameplay
}

fn presentation_only(n: &str) -> bool {
    const KEYS: &[&str] = &[
        "Camera", "DoF", "Palette", "Sound", "Bgm", "Voice", "XLink", "ELink", "Effect", "Rumble", "Fade", "LookAt", "Turn", "PlayAS", "Wait", "Navi", "Warp", "MoveTo", "Display", "Ducking", "Model", "Skip",
    ];
    KEYS.iter().any(|k| n.contains(k))
}

const Q_DATA: &[&str] = &[
    "EventQueryIsGameDataBool",
    "EventQueryIsGameDataBoolExp",
    "EventQueryIsGameDataInt",
    "EventQueryGameDataIntBranch",
    "EventQueryCompareGameDataInt",
    "EventQueryCompareGameDataFloat",
    "EventQueryIsGameDataFloat",
    "EventQueryIsGameDataEnum",
    "EventQueryIsGameDataString64",
    "EventQueryIsEmptyGameDataString64",
    "EventQueryIsBool",
    "EventQueryHavePlayedEvent",
    "EventQueryCheckIsGet",
    "EventQueryGetHavePlayedGetEvent",
    "EventQueryHavePlayedGetEventAtShop",
    "EventQueryIsObtainedSpiritBigPoeCount",
    "EventQueryIsGoBackHome",
    "EventQueryGameDataCheckStrayDogTakenAway",
    "EventQueryCheckElapsedTime",
    "EventQueryHaveRecordedTime",
    "EventQueryIsNotVisitCurrentBaseLocation",
];

const Q_POUCH: &[&str] = &[
    "EventQueryIsExistInPouch",
    "EventQueryIsExistInPouchWithNameGameData",
    "EventQueryHasRuppe",
    "EventQueryHeartPieceNum",
    "EventQueryTokenGoldNum",
    "EventQueryCheckTagOnPouchActor",
];

const Q_BOTTLE: &[&str] = &[
    "EventQueryCanAddToPorch",
    "EventQueryCanAddToPorchByGameData",
    "EventQueryHasAnyBottle",
    "EventQueryIsAnyBottleEmpty",
    "EventQueryIsExistBottleTypeInPouch",
    "EventQueryGetBottleContent",
    "EventQueryCheckLoseMoneyIfSellSome",
    "EventQueryCheckLoseMoneyIfSell",
    "EventQueryIsExistInPouchWithNameGameData",
];

const IMPL_ACTIONS: &[&str] = &[
    "EventTriggerSetGameDataBool",
    "EventTriggerSetGameDataInt",
    "EventTriggerSetGameDataEnum",
    "EventTriggerSetGameDataString64",
    "EventTriggerAddOrSubGameDataInt",
    "EventTriggerCopyGameDataInt",
    "EventTriggerCopyGameDataString64",
    "EventTriggerAddOrRemovePouchContent",
    "EventTriggerIncreaseRupee",
    "EventTriggerAddGameDataIntToRupee",
    "EventExecuteProgressChallengeStep",
    "EventTriggerProgressChallengeStep",
    "EventExecuteRollbackChallengeStep",
    "EventTransition",
    "EventTriggerSetRecentTalkFlag",
    "EventTriggerForceSetRecentTalkFlag",
    "EventDisableRecentTalkFlag",
    "EventTriggerSetMaskTalkFlag",
    "EventTriggerForceSetMaskTalkFlag",
    "EventTriggerForceSetAmiiboMaskTalkFlag",
    "EventTriggerSetVolatileGeneralPurposeFlag",
];

const IMPL_QUERIES: &[&str] = &[
    "EventQueryIsGameDataBool",
    "EventQueryIsGameDataBoolExp",
    "EventQueryIsGameDataInt",
    "EventQueryGameDataIntBranch",
    "EventQueryIsGameDataEnum",
    "EventQueryIsGameDataString64",
    "EventQueryIsBool",
    "EventQueryHavePlayedEvent",
    "EventQueryCheckIsGet",
    "EventQueryIsExistInPouch",
    "EventQueryHasRuppe",
    "EventQueryHeartPieceNum",
    "EventQueryPlayerIsChild",
    "EventQueryIsOnRecentTalkFlag",
    "EventQueryIsOnMaskTalkFlag",
    "EventQueryIsOnAmiiboMaskTalkFlag",
    "EventQueryVolatileGeneralPurposeFlagOn",
    "EventQueryHaveTalked",
    "EventQueryCheckHaveTalked",
];

pub fn implemented(n: &str, is_query: bool) -> bool {
    if is_query {
        return IMPL_QUERIES.contains(&n) || n.starts_with("EventQueryTalkChoice") || n.contains("RandomChoice");
    }
    IMPL_ACTIONS.contains(&n) || n.starts_with("EventReserveBancChange") || TALK.contains(&n)
}

pub fn query(n: &str) -> Class {
    if Q_BOTTLE.contains(&n) {
        return Class::QBottle;
    }
    if Q_DATA.contains(&n) {
        return Class::QData;
    }
    if Q_POUCH.contains(&n) {
        return Class::QPouch;
    }
    if n.starts_with("EventQueryTalkChoice") {
        return Class::QChoice;
    }
    if n.contains("RandomChoice") {
        return Class::QRandom;
    }
    if matches!(
        n,
        "EventQueryTimeDivision" | "EventQueryCheckTime" | "EventQueryIsCurrentTimeInSpecifiedTimeRange" | "EventQueryGreetingTimeDivision" | "EventQueryIsSunSongMakesMiddleTimeFromCurrentBanc"
    ) {
        return Class::QTime;
    }
    if matches!(
        n,
        "EventQueryHaveTalked"
            | "EventQueryCheckHaveTalked"
            | "EventQueryIsOnRecentTalkFlag"
            | "EventQueryIsOnMaskTalkFlag"
            | "EventQueryIsOnAmiiboMaskTalkFlag"
            | "EventQueryVolatileGeneralPurposeFlagOn"
    ) {
        return Class::QNpc;
    }
    if n.starts_with("EventQueryPlayerIsEquip") || n.starts_with("EventQueryPlayerIs") || n == "EventQueryCheckPlayerState" || n == "EventQueryCheckIsEquippedDynamicEquipment" || n == "EventQueryIsDreamWorld" {
        return Class::QPlayer;
    }
    const UI: &[&str] = &[
        "EventQueryIsHeartPiece",
        "EventQueryIsSmallKey",
        "EventQueryIsFairy",
        "EventQueryIsRecoverHeartOrMpActor",
        "EventQueryCheckPouchContentCategory",
        "EventQueryCheckPouchContentCategoryFromGameData",
        "EventQueryIsAutoSavePouchContent",
        "EventQueryCheckUIMessageTargetBottleActorName",
        "EventQueryCheckUIMessageTargetActorName",
        "EventQueryPouchContentGetType",
        "EventQueryIsMaxLifeUpPouchContent",
        "EventQueryIsOpenActionWindow",
        "EventQueryIsOpenActionWindowFromGameData",
        "EventQueryIsRupeeMax",
        "EventQueryHaveOpenedActionGuide",
        "EventQueryGetActionGuidePattern",
        "EventQueryIsInGameBgmSuspend",
        "EventQueryIsPlayingBgmName",
    ];
    if UI.contains(&n) {
        return Class::QScene;
    }
    const SCENE: &[&str] = &["Distance", "AISchedule", "Camera", "LookIK", "PlayingSameAS", "Front", "Turn", "Indoor", "Navi", "Emotion", "Dir", "Posture", "Actor", "Occlusion"];
    if SCENE.iter().any(|p| n.contains(p)) {
        return Class::QScene;
    }
    Class::QEngine
}

#[derive(Clone, Debug, Default)]
pub struct Census {
    pub files: usize,
    pub flows: usize,
    pub events: usize,
    pub kinds: Map<String, usize>,
    pub by_class: Map<Class, usize>,
    pub impl_events: usize,
    pub impl_by_class: Map<Class, usize>,
    pub actions: Map<String, usize>,
    pub queries: Map<String, usize>,
    pub actors: Map<String, usize>,
    pub subs: Map<String, usize>,
    pub tls: usize,
    pub clips: usize,
    pub shots: usize,
    pub cuts: usize,
    pub tl_by_class: Map<Class, usize>,
    pub tl_actions: Map<String, usize>,
}

impl Census {
    pub fn add(&mut self, b: &Bf) {
        self.files += 1;
        for f in &b.flows {
            self.flow(f);
        }
        for t in &b.tls {
            self.tls += 1;
            self.cuts += t.cuts.len();
            let mut names: Vec<String> = Vec::new();
            for c in &t.clips {
                self.clips += 1;
                names.push(t.actors.get(c.actor as usize).and_then(|a| a.actions.get(c.act as usize)).cloned().unwrap_or_default());
            }
            for c in &t.shots {
                self.shots += 1;
                names.push(t.actors.get(c.actor as usize).and_then(|a| a.actions.get(c.act as usize)).cloned().unwrap_or_default());
            }
            for n in names {
                *self.tl_by_class.entry(action(&n)).or_insert(0) += 1;
                *self.tl_actions.entry(n).or_insert(0) += 1;
            }
        }
    }

    fn flow(&mut self, f: &Flow) {
        self.flows += 1;
        for a in &f.actors {
            *self.actors.entry(a.name.clone()).or_insert(0) += 1;
        }
        for e in &f.events {
            self.events += 1;
            let (k, c): (&str, Class) = match &e.kind {
                Kind::Action { actor, act, .. } => {
                    let n: String = f
                        .actors
                        .get(*actor as usize)
                        .and_then(|a| a.actions.get(*act as usize))
                        .cloned()
                        .unwrap_or_default();
                    *self.actions.entry(n.clone()).or_insert(0) += 1;
                    if implemented(&n, false) {
                        self.impl_events += 1;
                        *self.impl_by_class.entry(action(&n)).or_insert(0) += 1;
                    }
                    ("action", action(&n))
                }
                Kind::Switch { actor, query: q, .. } => {
                    let n: String = f
                        .actors
                        .get(*actor as usize)
                        .and_then(|a| a.queries.get(*q as usize))
                        .cloned()
                        .unwrap_or_default();
                    *self.queries.entry(n.clone()).or_insert(0) += 1;
                    if implemented(&n, true) {
                        self.impl_events += 1;
                        *self.impl_by_class.entry(query(&n)).or_insert(0) += 1;
                    }
                    ("switch", query(&n))
                }
                Kind::Sub { flow, entry, .. } => {
                    *self.subs.entry(format!("{flow}:{entry}")).or_insert(0) += 1;
                    ("sub", Class::Control)
                }
                Kind::SubSwitch { .. } => ("subswitch", Class::Control),
                Kind::Fork { .. } => ("fork", Class::Control),
                Kind::Join { .. } => ("join", Class::Control),
                Kind::Label { .. } => ("label", Class::Control),
            };
            if c == Class::Control {
                self.impl_events += 1;
                *self.impl_by_class.entry(c).or_insert(0) += 1;
            }
            *self.kinds.entry(k.to_string()).or_insert(0) += 1;
            *self.by_class.entry(c).or_insert(0) += 1;
        }
    }

    pub fn fit_totals(&self) -> Map<Fit, usize> {
        let mut out: Map<Fit, usize> = Map::new();
        for (c, n) in &self.by_class {
            *out.entry(c.fit()).or_insert(0) += n;
        }
        out
    }

    pub fn report(&self) -> String {
        let mut o: String = String::new();
        o.push_str(&format!("files {} flows {} events {}\n", self.files, self.flows, self.events));
        for (k, v) in &self.kinds {
            o.push_str(&format!("kind {k} {v}\n"));
        }
        o.push_str(&format!("distinct actor types {} actions {} queries {}\n", self.actors.len(), self.actions.len(), self.queries.len()));
        for (c, n) in &self.by_class {
            o.push_str(&format!("class {:?} | {} | {:?} | {} | {:.1}%\n", c, c.label(), c.fit(), n, 100.0 * *n as f64 / self.events as f64));
        }
        o.push_str(&format!("implemented in the prototype {} events {:.1}%
", self.impl_events, 100.0 * self.impl_events as f64 / self.events as f64));
        for (c, n) in &self.impl_by_class {
            o.push_str(&format!("impl {:?} {}
", c, n));
        }
        for (f, n) in self.fit_totals() {
            o.push_str(&format!("fit {:?} {} {:.1}%\n", f, n, 100.0 * n as f64 / self.events as f64));
        }
        o.push_str(&format!("timelines {} clips {} shots {} cuts {} distinct clip actions {}
", self.tls, self.clips, self.shots, self.cuts, self.tl_actions.len()));
        for (c, n) in &self.tl_by_class {
            o.push_str(&format!("tlclass {:?} {} {:?}
", c, n, c.fit()));
        }
        let mut names: Map<Class, Vec<(&String, &usize)>> = Map::new();
        for (n, c) in &self.actions {
            names.entry(action(n)).or_default().push((n, c));
        }
        for (n, c) in &self.queries {
            names.entry(query(n)).or_default().push((n, c));
        }
        for (c, v) in names.iter_mut() {
            v.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
            o.push_str(&format!("names {:?} {}:", c, v.len()));
            for (n, k) in v.iter().take(400) {
                o.push_str(&format!(" {n}={k}"));
            }
            o.push('\n');
        }
        o
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_names_land_in_their_classes() {
        assert_eq!(action("EventTriggerSetGameDataBool"), Class::DataSet);
        assert_eq!(action("EventExecuteProgressChallengeStep"), Class::Challenge);
        assert_eq!(action("EventMoveToTargetPosCamera"), Class::Inert);
        assert_eq!(action("EventReserveBancChangeForFaroreWind"), Class::Leave);
        assert_eq!(query("EventQueryIsGameDataBoolExp"), Class::QData);
        assert_eq!(query("EventQueryTalkChoice2"), Class::QChoice);
        assert_eq!(query("EventQueryCameraCheckRayCastForMoveToTarget"), Class::QScene);
        assert_eq!(Class::QData.fit(), Fit::Exact);
        assert_eq!(Class::QScene.fit(), Fit::Opaque);
    }
}
