use crate::byml::{Byml, Node};
use crate::error::{Error as E, Res};
use crate::pickup_pack::closure::{actor_file, chain, local, norm, work, Chain, Pool, COMPONENTS, PARENT};
use crate::pickup_pack::magic;
use crate::pickup_pack::{is_actor_name, Layers, Spec, Style, BASE, DONOR};

pub const NAME_REF: &str = "ActorNameRef";
pub const MODEL_REF: &str = "ModelInfoRef";
pub const ANIM_REF: &str = "AnimationRef";
pub const AS_REF: &str = "ASRef";
pub const AS_INFO_REF: &str = "ASInfoRef";
pub const POUCH_REF: &str = "PouchContentRef";
pub const ATTENTION_REF: &str = "AttentionSenderRef";
pub const POUCH_ACTOR: &str = "PouchActor";
pub const FIRST_CHECK: &str = "IsCountCheckOnFirstGet";
pub const AUTO_GET: &str = "DefaultAutoGet";
pub const SHARED_ANIM: &str = "Item_Common_Animation";
const ANIM_LIST: &str = "AnimationResources";
const ANIM_NAME: &str = "ModelProjectName";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Made {
    pub actor: (String, Vec<u8>),
    pub pouch: (String, Vec<u8>),
    pub identity: String,
    pub extra: Vec<(String, Vec<u8>)>,
}

pub fn pouch_file(name: &str) -> String {
    format!("Component/PouchContentParam/{name}.game__component__PouchContentParam.bgyml")
}

fn need<'a>(c: &'a Chain, who: &str, k: &str) -> Res<&'a str> {
    c.comps.get(k).map(|s| s.as_str()).ok_or_else(|| E::Msg(format!("{who} has no {k}, this template cannot carry it")))
}

fn held(p: &Pool, who: &str, k: &str, v: &str) -> Res<String> {
    let f: String = norm(v).ok_or_else(|| E::Msg(format!("{who} has a {k} that is not a file reference")))?;
    if p.get(&f).is_none() {
        return Err(E::Msg(format!("{who} points its {k} at {f}, which is not in its pack")));
    }
    Ok(f)
}

pub fn shared_anim(p: &Pool, c: &Chain) -> bool {
    let Some(f) = c.comps.get(ANIM_REF).and_then(|v| norm(v)) else {
        return false;
    };
    let Ok(y) = p.byml(&f) else {
        return false;
    };
    y.root.get(ANIM_LIST).and_then(|v| v.as_arr()).and_then(|a| a.first()).and_then(|x| x.get(ANIM_NAME)).and_then(|v| v.as_str()) == Some(SHARED_ANIM)
}

pub fn fits(p: &Pool, s: &Spec) -> Res<Vec<String>> {
    let item: Chain = chain(p, &s.item)?;
    let donor: Chain = chain(p, DONOR)?;
    let mut bad: Vec<String> = Vec::new();
    if item.files.contains(&actor_file(BASE)) {
        bad.push(format!("{} is already a world pickup", s.item));
    }
    for k in [NAME_REF, MODEL_REF, ANIM_REF, POUCH_REF] {
        match item.comps.get(k) {
            None => bad.push(format!("{} has no {k}", s.item)),
            Some(v) if k != NAME_REF && held(p, &s.item, k, v).is_err() => bad.push(format!("{} keeps its {k} outside its pack", s.item)),
            Some(_) => {}
        }
    }
    if item.comps.get(AS_REF) != donor.comps.get(AS_REF) && !shared_anim(p, &item) {
        bad.push(format!("{} does not use the animation set of the shipped pickups", s.item));
    }
    Ok(bad)
}

fn actor_in(r: &str) -> Option<String> {
    let file: &str = r.rsplit('/').next().unwrap_or(r);
    let name: &str = file.split(".engine__actor__").next().unwrap_or(file);
    is_actor_name(name).then(|| name.to_string())
}

pub fn identity(p: &Pool, s: &Spec, item: &Chain, pouch: &str) -> Res<String> {
    let mut seen: Vec<String> = Vec::new();
    let mut cur: Option<String> = Some(pouch.to_string());
    while let Some(f) = cur {
        if seen.contains(&f) || p.get(&f).is_none() {
            break;
        }
        let y: Byml = p.byml(&f)?;
        if let Some(v) = y.root.get(POUCH_ACTOR).and_then(|v| v.as_str()) {
            return actor_in(v).ok_or_else(|| E::Msg(format!("{f} names the pouch actor {v}, which is not an actor file")));
        }
        cur = y.root.get(PARENT).and_then(|v| v.as_str()).and_then(norm);
        seen.push(f);
    }
    Ok(item.comps.get(NAME_REF).and_then(|v| actor_in(v)).unwrap_or_else(|| s.item.clone()))
}

pub fn make(p: &Pool, s: &Spec) -> Res<Made> {
    make_with(p, s, &Layers::default())
}

pub fn make_with(p: &Pool, s: &Spec, l: &Layers) -> Res<Made> {
    let item: Chain = chain(p, &s.item)?;
    let donor: Chain = chain(p, DONOR)?;
    if let Some(m) = fits(p, s)?.first() {
        return Err(E::Msg(format!("{m}, so the thin pickup template does not fit")));
    }
    let item_pouch: String = held(p, &s.item, POUCH_REF, need(&item, &s.item, POUCH_REF)?)?;
    let version: u16 = p.byml(&actor_file(DONOR))?.version;
    let pouch_path: String = pouch_file(&s.name);
    let mut comps: Node = Node::Map(Vec::new());
    comps.set(AS_INFO_REF, Node::Str(need(&donor, DONOR, AS_INFO_REF)?.to_string()));
    comps.set(AS_REF, Node::Str(need(&donor, DONOR, AS_REF)?.to_string()));
    comps.set(NAME_REF, Node::Str(need(&item, &s.item, NAME_REF)?.to_string()));
    comps.set(ANIM_REF, Node::Str(need(&item, &s.item, ANIM_REF)?.to_string()));
    comps.set(MODEL_REF, Node::Str(need(&item, &s.item, MODEL_REF)?.to_string()));
    comps.set(POUCH_REF, Node::Str(local(&pouch_path)));
    if s.style == Style::Press {
        comps.set(ATTENTION_REF, Node::Str(need(&donor, DONOR, ATTENTION_REF)?.to_string()));
    }
    let mut extra: Vec<(String, Vec<u8>)> = Vec::new();
    if l.magic {
        let (f, d, r): (String, Vec<u8>, String) = magic::make(p, &item, &chain(p, BASE)?, &s.item, &s.name)?;
        comps.set(magic::TABLE_REF, Node::Str(r));
        extra.push((f, d));
    }
    let mut actor: Node = Node::Map(Vec::new());
    actor.set(PARENT, Node::Str(work(&actor_file(BASE))));
    actor.set(COMPONENTS, comps);
    let mut pouch: Node = Node::Map(Vec::new());
    pouch.set(PARENT, Node::Str(work(&item_pouch)));
    let who: String = identity(p, s, &item, &item_pouch)?;
    pouch.set(POUCH_ACTOR, Node::Str(work(&actor_file(&who))));
    pouch.set(FIRST_CHECK, Node::Bool(false));
    if s.style == Style::Touch {
        pouch.set(AUTO_GET, Node::Bool(true));
    }
    Ok(Made {
        actor: (actor_file(&s.name), Byml { root: actor, version, big: false }.write()?),
        pouch: (pouch_path, Byml { root: pouch, version, big: false }.write()?),
        identity: who,
        extra,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn file(pairs: &[(&str, Node)]) -> Vec<u8> {
        let mut root: Node = Node::Map(Vec::new());
        for (k, v) in pairs {
            root.set(k, v.clone());
        }
        Byml { root, version: 7, big: false }.write().unwrap()
    }

    fn s(x: &str) -> Node {
        Node::Str(x.to_string())
    }

    fn comps(pairs: &[(&str, &str)]) -> Node {
        let mut m: Node = Node::Map(Vec::new());
        for (k, v) in pairs {
            m.set(k, s(v));
        }
        m
    }

    pub(crate) fn pool() -> Pool {
        let mut p: Pool = Pool::default();
        let one: Vec<u8> = file(&[("A", Node::Bool(true))]);
        p.add(
            &actor_file(BASE),
            &file(&[(
                COMPONENTS,
                comps(&[
                    (ATTENTION_REF, "?Component/Attention/Auto.game__component__AttentionSenderParam.bgyml"),
                    (MODEL_REF, "?Component/ModelInfo/Dummy.engine__component__ModelInfo.bgyml"),
                    (POUCH_REF, "?Component/PouchContentParam/Obj_Item_Base.game__component__PouchContentParam.bgyml"),
                    ("PhysicsRef", "?Component/Physics/Obj_Item_Base.engine__component__PhysicsParam.bgyml"),
                ]),
            )]),
        )
        .unwrap();
        p.add("Component/Attention/Auto.game__component__AttentionSenderParam.bgyml", &one).unwrap();
        p.add("Component/ModelInfo/Dummy.engine__component__ModelInfo.bgyml", &one).unwrap();
        p.add("Component/PouchContentParam/Obj_Item_Base.game__component__PouchContentParam.bgyml", &one).unwrap();
        p.add("Component/Physics/Obj_Item_Base.engine__component__PhysicsParam.bgyml", &one).unwrap();
        p.add(
            &actor_file(DONOR),
            &file(&[
                (PARENT, s(&work(&actor_file(BASE)))),
                (
                    COMPONENTS,
                    comps(&[
                        (AS_INFO_REF, "?Component/ASInfo/Item_Obj.engine__component__ASInfo.bgyml"),
                        (AS_REF, "?AS/Item_Common.root.asb"),
                        (ATTENTION_REF, "?Component/Attention/Press.game__component__AttentionSenderParam.bgyml"),
                        (NAME_REF, "?ActorSystem/ActorName/Item_Arrow_Fire.engine__actor__ActorName.bgyml"),
                    ]),
                ),
            ]),
        )
        .unwrap();
        p.add("Component/ASInfo/Item_Obj.engine__component__ASInfo.bgyml", &one).unwrap();
        p.add("Component/Attention/Press.game__component__AttentionSenderParam.bgyml", &one).unwrap();
        p.add(
            &actor_file("Item_Test"),
            &file(&[(
                COMPONENTS,
                comps(&[
                    (AS_REF, "?AS/Item_Common.root.asb"),
                    (NAME_REF, "?ActorSystem/ActorName/Item_Test.engine__actor__ActorName.bgyml"),
                    (ANIM_REF, "?Component/AnimationParam/Item_Test.engine__component__AnimationParam.bgyml"),
                    ("CaptureRef", "?Component/CaptureParam/Item_Test.game__component__CaptureParam.bgyml"),
                    (MODEL_REF, "?Component/ModelInfo/Item_Test.engine__component__ModelInfo.bgyml"),
                    (POUCH_REF, "?Component/PouchContentParam/Item_Test.game__component__PouchContentParam.bgyml"),
                ]),
            )]),
        )
        .unwrap();
        p.add("Component/AnimationParam/Item_Test.engine__component__AnimationParam.bgyml", &one).unwrap();
        p.add("Component/CaptureParam/Item_Test.game__component__CaptureParam.bgyml", &one).unwrap();
        p.add("Component/ModelInfo/Item_Test.engine__component__ModelInfo.bgyml", &one).unwrap();
        p.add("Component/PouchContentParam/Item_Test.game__component__PouchContentParam.bgyml", &one).unwrap();
        p
    }

    pub(crate) fn spec(style: Style) -> Spec {
        Spec { item: "Item_Test".to_string(), name: "Obj_Test".to_string(), style, heap: None }
    }

    #[test]
    fn the_new_actor_is_a_child_of_the_pickup_base_that_keeps_the_item_identity() {
        let m: Made = make(&pool(), &spec(Style::Touch)).unwrap();
        assert_eq!(m.actor.0, "Actor/Obj_Test.engine__actor__ActorParam.bgyml");
        let y: Byml = Byml::parse(&m.actor.1).unwrap();
        assert_eq!(y.version, 7);
        assert_eq!(y.root.get(PARENT).and_then(|v| v.as_str()), Some("Work/Actor/Obj_Item_Base.engine__actor__ActorParam.gyml"));
        let c: &Node = y.root.get(COMPONENTS).unwrap();
        let text = |k: &str| c.get(k).and_then(|v| v.as_str()).map(|x| x.to_string());
        assert_eq!(text(NAME_REF).as_deref(), Some("?ActorSystem/ActorName/Item_Test.engine__actor__ActorName.bgyml"));
        assert_eq!(text(MODEL_REF).as_deref(), Some("?Component/ModelInfo/Item_Test.engine__component__ModelInfo.bgyml"));
        assert_eq!(text(POUCH_REF).as_deref(), Some("?Component/PouchContentParam/Obj_Test.game__component__PouchContentParam.bgyml"));
        assert_eq!(text(AS_INFO_REF).as_deref(), Some("?Component/ASInfo/Item_Obj.engine__component__ASInfo.bgyml"));
        assert_eq!(text(ATTENTION_REF), None);
        assert_eq!(text("PhysicsRef"), None);
        assert_eq!(text("CaptureRef"), None);
    }

    #[test]
    fn the_pouch_file_inherits_the_item_and_names_it_as_the_pouch_actor() {
        let m: Made = make(&pool(), &spec(Style::Touch)).unwrap();
        let y: Byml = Byml::parse(&m.pouch.1).unwrap();
        assert_eq!(y.root.get(PARENT).and_then(|v| v.as_str()), Some("Work/Component/PouchContentParam/Item_Test.game__component__PouchContentParam.gyml"));
        assert_eq!(y.root.get(POUCH_ACTOR).and_then(|v| v.as_str()), Some("Work/Actor/Item_Test.engine__actor__ActorParam.gyml"));
        assert_eq!(y.root.get(FIRST_CHECK), Some(&Node::Bool(false)));
        assert_eq!(y.root.get(AUTO_GET), Some(&Node::Bool(true)));
        assert_eq!(y.root.get("SpecialDeal"), None);
    }

    #[test]
    fn the_press_style_takes_the_button_attention_and_drops_auto_get() {
        let m: Made = make(&pool(), &spec(Style::Press)).unwrap();
        let a: Byml = Byml::parse(&m.actor.1).unwrap();
        let got: Option<&str> = a.root.get(COMPONENTS).and_then(|c| c.get(ATTENTION_REF)).and_then(|v| v.as_str());
        assert_eq!(got, Some("?Component/Attention/Press.game__component__AttentionSenderParam.bgyml"));
        let y: Byml = Byml::parse(&m.pouch.1).unwrap();
        assert_eq!(y.root.get(AUTO_GET), None);
    }

    #[test]
    fn an_item_with_another_animation_set_or_a_missing_part_is_refused() {
        let mut p: Pool = pool();
        let odd: Vec<u8> = file(&[(
            COMPONENTS,
            comps(&[
                (AS_REF, "?AS/Weapon_Sword.root.asb"),
                (NAME_REF, "?ActorSystem/ActorName/Item_Odd.engine__actor__ActorName.bgyml"),
                (MODEL_REF, "?Component/ModelInfo/Item_Test.engine__component__ModelInfo.bgyml"),
            ]),
        )]);
        p.add(&actor_file("Item_Odd"), &odd).unwrap();
        let sp: Spec = Spec { item: "Item_Odd".to_string(), name: "Obj_Odd".to_string(), style: Style::Touch, heap: None };
        let bad: Vec<String> = fits(&p, &sp).unwrap();
        assert_eq!(bad.len(), 3);
        assert!(make(&p, &sp).is_err());
        let own: Spec = Spec { item: DONOR.to_string(), name: "Obj_Twice".to_string(), style: Style::Touch, heap: None };
        assert!(fits(&p, &own).unwrap().iter().any(|m| m.contains("already a world pickup")));
    }

    fn anim(first: &str) -> Vec<u8> {
        let mut a: Node = Node::Map(Vec::new());
        a.set(ANIM_NAME, s(first));
        let mut b: Node = Node::Map(Vec::new());
        b.set(ANIM_NAME, s("Item_Long_Animation"));
        file(&[(ANIM_LIST, Node::Arr(vec![a, b]))])
    }

    fn long(p: &mut Pool, name: &str, first: &str) -> Spec {
        let f: String = format!("Component/AnimationParam/{name}.engine__component__AnimationParam.bgyml");
        p.add(&f, &anim(first)).unwrap();
        let y: Vec<u8> = file(&[(
            COMPONENTS,
            comps(&[
                (AS_REF, "?AS/Pouch_Long.root.asb"),
                (NAME_REF, "?ActorSystem/ActorName/Item_Long.engine__actor__ActorName.bgyml"),
                (ANIM_REF, &format!("?{f}")),
                (MODEL_REF, "?Component/ModelInfo/Item_Test.engine__component__ModelInfo.bgyml"),
                (POUCH_REF, "?Component/PouchContentParam/Item_Test.game__component__PouchContentParam.bgyml"),
            ]),
        )]);
        p.add(&actor_file(name), &y).unwrap();
        Spec { item: name.to_string(), name: format!("Obj_{name}"), style: Style::Touch, heap: None }
    }

    #[test]
    fn another_animation_set_is_accepted_only_when_the_shared_animations_come_first() {
        let mut p: Pool = pool();
        let good: Spec = long(&mut p, "Item_Long", SHARED_ANIM);
        assert!(fits(&p, &good).unwrap().is_empty());
        let m: Made = make(&p, &good).unwrap();
        let y: Byml = Byml::parse(&m.actor.1).unwrap();
        assert_eq!(y.root.get(COMPONENTS).and_then(|c| c.get(AS_REF)).and_then(|v| v.as_str()), Some("?AS/Item_Common.root.asb"));
        let second: Spec = long(&mut p, "Item_Late", "Item_Long_Animation");
        let bad: Vec<String> = fits(&p, &second).unwrap();
        assert_eq!(bad.len(), 1);
        assert!(bad[0].contains("animation set"));
        assert!(make(&p, &second).is_err());
        assert!(!shared_anim(&p, &chain(&p, "Item_Test").unwrap()));
        assert!(fits(&p, &spec(Style::Touch)).unwrap().is_empty());
    }

    #[test]
    fn the_magic_layer_is_off_unless_asked_for() {
        let p: Pool = pool();
        let m: Made = make(&p, &spec(Style::Touch)).unwrap();
        assert!(m.extra.is_empty());
        assert_eq!(make_with(&p, &spec(Style::Touch), &Layers::default()).unwrap(), m);
        let y: Byml = Byml::parse(&m.actor.1).unwrap();
        assert!(y.root.get(COMPONENTS).and_then(|c| c.get(magic::TABLE_REF)).is_none());
        let on: Layers = Layers { magic: true, ..Layers::default() };
        assert!(make_with(&p, &spec(Style::Touch), &on).is_err());
    }
}
