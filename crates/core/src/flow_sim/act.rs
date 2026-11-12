use crate::bfevfl::Cont;
use crate::flags::hash_name;
use crate::flow_sim::ask::{Pr, RUPEE};
use crate::flow_sim::expr;
use crate::flow_sim::run::{push, Env, Mach, Sim, RK};
use crate::flow_sim::state::{State, V};
use crate::flow_sim::vocab::Class;

fn not_ready() -> u32 {
    hash_name("NotReady")
}

fn ready() -> u32 {
    hash_name("Ready")
}

fn complete() -> u32 {
    hash_name("Complete")
}

impl Sim {
    pub fn put(&mut self, m: &mut Mach, n: &str, v: V) {
        let h: u32 = self.db.reg(n);
        if !self.db.defs.contains_key(&h) {
            self.unknown.insert(n.to_string());
        }
        let old: Option<V> = self.db.get(&m.st, h);
        let shown: String = self.db.show(h, &v);
        let was: String = match &old {
            Some(o) => self.db.show(h, o),
            None => "?".to_string(),
        };
        push(m, RK::Set, format!("write {n} = {shown} (was {was})"));
        m.writes.insert(n.to_string(), shown);
        m.st.v.insert(h, v);
    }

    fn read_int(&mut self, m: &Mach, n: &str) -> i64 {
        let h: u32 = self.db.reg(n);
        self.db.int(&m.st, h).unwrap_or(0)
    }

    fn grant(&mut self, m: &mut Mach, item: &str, n: i64, ev: bool) {
        *m.st.grants.entry(item.to_string()).or_insert(0) += n;
        *m.gr.entry(item.to_string()).or_insert(0) += n;
        let sn: String = format!("PouchData.{item}.StockNum");
        let h: u32 = self.db.reg(&sn);
        if self.db.defs.contains_key(&h) {
            let cur: i64 = self.db.int(&m.st, h).unwrap_or(0);
            m.st.v.insert(h, V::I(cur + n));
        }
        let ig: String = format!("IsGet.{item}");
        let hi: u32 = self.db.reg(&ig);
        if self.db.defs.contains_key(&hi) && n > 0 {
            m.st.v.insert(hi, V::B(true));
        }
        if ev {
            let hp: u32 = self.db.reg(&format!("HavePlayedGetEvent.{item}"));
            if self.db.defs.contains_key(&hp) {
                m.st.v.insert(hp, V::B(true));
            }
        }
        push(m, RK::Grant, format!("grant {item} x{n}"));
    }

    pub fn request(&mut self, m: &mut Mach, chal: &str, step: &str) {
        let sh: u32 = self.db.reg(&format!("Step_{chal}"));
        let cur: Option<V> = self.db.get(&m.st, sh);
        let Some(V::E(c)) = cur else {
            m.reqs.push(format!("{chal} -> {step}: no such challenge"));
            push(m, RK::Warn, format!("challenge {chal} is not a known Step enum"));
            return;
        };
        let cn: String = self.db.enum_name(sh, c);
        let valid: bool = self.db.defs.get(&sh).map(|d| d.vals.iter().any(|v| v == step)).unwrap_or(false);
        if !valid {
            m.reqs.push(format!("{chal} -> {step}: no such step"));
            push(m, RK::Warn, format!("challenge {chal} has no step {step}"));
            return;
        }
        if c == not_ready() || c == complete() {
            m.reqs.push(format!("{chal} -> {step}: dropped, challenge is {cn}"));
            push(m, RK::Chal, format!("request {chal} -> {step} DROPPED, the challenge is {cn}"));
            return;
        }
        if c == hash_name(step) {
            m.reqs.push(format!("{chal} -> {step}: already there"));
            push(m, RK::Chal, format!("request {chal} -> {step} is a no-op, already there"));
            return;
        }
        m.reqs.push(format!("{chal} -> {step}: accepted from {cn}"));
        push(m, RK::Chal, format!("request {chal} -> {step} accepted (was {cn})"));
        m.st.v.insert(sh, V::E(hash_name(step)));
        m.writes.insert(format!("Step_{chal}"), step.to_string());
    }

    pub fn own(&mut self, st: &mut State, item: &str, n: i64) {
        let h: u32 = self.db.reg(&format!("PouchData.{item}.StockNum"));
        st.v.insert(h, V::I(n));
        let g: u32 = self.db.reg(&format!("IsGet.{item}"));
        if self.db.defs.contains_key(&g) {
            st.v.insert(g, V::B(n > 0));
        }
        st.grants.remove(item);
    }

    pub fn settle(&self, st: &mut State) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for c in &self.db.chals {
            let cur: Option<V> = self.db.get(st, c.step);
            if cur != Some(V::E(not_ready())) {
                continue;
            }
            let ok: bool = match c.dep {
                None => true,
                Some(d) => expr::truth(&self.db, st, d, 0) == Some(true),
            };
            if ok {
                st.v.insert(c.step, V::E(ready()));
                out.push(format!("challenge manager: {} NotReady -> Ready", c.name));
            }
        }
        out
    }

    pub fn tick(&mut self, m: &mut Mach) {
        let t: Vec<String> = self.settle(&mut m.st);
        for x in t {
            push(m, RK::Chal, x);
        }
    }

    pub fn check(&mut self, m: &mut Mach) {
        for (t, lits) in self.inv.clone() {
            let hit: bool = lits.iter().all(|(want, h)| crate::flow_sim::expr::truth(&self.db, &m.st, *h, 0) == Some(*want));
            if hit && !m.viol.contains(&t) {
                m.viol.push(t.clone());
                push(m, RK::Warn, format!("INVARIANT BROKEN: never ({t}) but it holds now"));
            }
        }
    }

    pub fn act(&mut self, m: &mut Mach, actor: &str, name: &str, c: &Option<Cont>, env: &Env, cls: Class, ev: &str) -> Option<(String, String)> {
        let p: Pr = Pr { env, c };
        let verbose: bool = self.opts.verbose;
        match name {
            "EventTriggerSetGameDataBool" => match (p.s("GameDataName"), p.b("Value"), p.i("Index").unwrap_or(-1)) {
                (Some(n), Some(v), -1) => self.put(m, &n, V::B(v)),
                _ => self.note(m, name, "array or unresolved target"),
            },
            "EventTriggerSetGameDataInt" => match (p.s("GameDataName"), p.i("Value")) {
                (Some(n), Some(v)) => self.put(m, &n, V::I(v)),
                _ => self.note(m, name, "unresolved"),
            },
            "EventTriggerSetGameDataEnum" => match (p.s("GameDataName"), p.s("Value")) {
                (Some(n), Some(v)) => self.put(m, &n, V::E(hash_name(&v))),
                _ => self.note(m, name, "unresolved"),
            },
            "EventTriggerSetGameDataString64" => match (p.s("GameDataName"), p.s("Value")) {
                (Some(n), Some(v)) => self.put(m, &n, V::S(v)),
                _ => self.note(m, name, "unresolved"),
            },
            "EventTriggerAddOrSubGameDataInt" => match (p.s("GameDataName"), p.i("Value")) {
                (Some(n), Some(v)) => {
                    let cur: i64 = self.read_int(m, &n);
                    self.put(m, &n, V::I(cur + v));
                }
                _ => self.note(m, name, "unresolved"),
            },
            "EventTriggerCopyGameDataInt" => match (p.s("SrcGameDataName"), p.s("DstGameDataName")) {
                (Some(a), Some(b)) => {
                    let cur: i64 = self.read_int(m, &a);
                    self.put(m, &b, V::I(cur));
                }
                _ => self.note(m, name, "unresolved"),
            },
            "EventTriggerCopyGameDataString64" => match (p.s("SrcGameDataName"), p.s("DstGameDataName")) {
                (Some(a), Some(b)) => {
                    let h: u32 = self.db.reg(&a);
                    let cur: V = self.db.get(&m.st, h).unwrap_or(V::S(String::new()));
                    self.put(m, &b, cur);
                }
                _ => self.note(m, name, "unresolved"),
            },
            "EventTriggerAddOrRemovePouchContent" => match p.s("Actor") {
                Some(a) => {
                    let n: i64 = p.i("Num").unwrap_or(1);
                    let ge: bool = p.b("IsGetEvent").unwrap_or(false);
                    self.grant(m, &a, n, ge);
                }
                None => self.note(m, name, "actor is not resolvable"),
            },
            "EventTriggerIncreaseRupee" => match p.i("Value") {
                Some(v) => {
                    let cur: i64 = self.read_int(m, RUPEE);
                    self.put(m, RUPEE, V::I((cur + v).max(0)));
                }
                None => self.note(m, name, "unresolved"),
            },
            "EventTriggerAddGameDataIntToRupee" => match p.s("GameDataName") {
                Some(n) => {
                    let v: i64 = self.read_int(m, &n);
                    let sign: i64 = if p.b("IsSignInverse").unwrap_or(false) { -1 } else { 1 };
                    let cur: i64 = self.read_int(m, RUPEE);
                    self.put(m, RUPEE, V::I((cur + sign * v).max(0)));
                }
                None => self.note(m, name, "unresolved"),
            },
            "EventExecuteProgressChallengeStep" | "EventTriggerProgressChallengeStep" => match (p.s("ChallengeName"), p.s("StepName")) {
                (Some(c), Some(s)) => self.request(m, &c, &s),
                _ => self.note(m, name, "unresolved"),
            },
            "EventExecuteRollbackChallengeStep" => match (p.s("ChallengeName"), p.s("ChallengeStep").or(p.s("StepName"))) {
                (Some(c), Some(s)) => self.request(m, &c, &s),
                _ => self.note(m, name, "unresolved"),
            },
            "EventTransition" => {
                let e: Option<String> = p.s("EventName");
                if let Some(e) = e {
                    push(m, RK::Trans, format!("transition to event {e}"));
                    m.trans.push(e.clone());
                    let n: String = format!("HavePlayedEvent.{e}");
                    self.put(m, &n, V::B(true));
                    if self.opts.follow {
                        if let Ok(Some((stem, entry))) = self.src.event(&e) {
                            if !stem.is_empty() && matches!(self.flow_id(&stem), Ok(Some(_))) {
                                return Some((stem, entry));
                            }
                        }
                    }
                }
            }
            "EventSuspend" | "EventRequestQuit" => push(m, RK::Trans, format!("{name}")),
            "EventTriggerSetRecentTalkFlag" | "EventTriggerForceSetRecentTalkFlag" => {
                m.st.npc.insert(format!("{actor}.recent"), true);
            }
            "EventDisableRecentTalkFlag" => {
                m.st.npc.insert(format!("{actor}.recent"), false);
            }
            "EventTriggerSetMaskTalkFlag" | "EventTriggerForceSetMaskTalkFlag" | "EventTriggerForceSetAmiiboMaskTalkFlag" => {
                m.st.npc.insert(format!("{actor}.mask"), true);
            }
            "EventTriggerSetVolatileGeneralPurposeFlag" => {
                m.st.npc.insert(format!("{actor}.volatile"), p.b("Value").unwrap_or(true));
            }
            _ => return self.other(m, name, cls, &p, ev, verbose),
        }
        None
    }

    fn note(&mut self, m: &mut Mach, name: &str, why: &str) {
        *self.assumed.entry(format!("skipped {name} ({why})")).or_insert(0) += 1;
        push(m, RK::Warn, format!("{name}: {why}, not applied"));
    }

    fn boot(&mut self, m: &mut Mach, e: &str) -> Option<(String, String)> {
        if e.is_empty() || !self.opts.follow {
            return None;
        }
        let n: String = format!("HavePlayedEvent.{e}");
        self.put(m, &n, V::B(true));
        match self.src.event(e) {
            Ok(Some((stem, entry))) if !stem.is_empty() && matches!(self.flow_id(&stem), Ok(Some(_))) => Some((stem, entry)),
            _ => None,
        }
    }

    fn other(&mut self, m: &mut Mach, name: &str, cls: Class, p: &Pr, ev: &str, verbose: bool) -> Option<(String, String)> {
        match cls {
            Class::Talk => {
                let id: String = p.s("MessageId").or(p.s("MessageID")).or(p.s("DefaultMessageID")).unwrap_or_default();
                let ch: i64 = p.i("ChoiceNumber").unwrap_or(0);
                if !id.is_empty() {
                    let t: String = if ch > 0 { format!("message {id} ({ch} choices)") } else { format!("message {id}") };
                    push(m, RK::Msg, t);
                }
            }
            Class::Leave => {
                if name.starts_with("EventReserveBancChange") {
                    let b: String = p.s("NextBanc").unwrap_or_default();
                    let e: String = p.s("BootEventName").unwrap_or_default();
                    push(m, RK::Scene, format!("scene change to {b} boot event {e}"));
                    m.trans.push(format!("scene:{b}:{e}"));
                    return self.boot(m, &e);
                } else {
                    push(m, RK::Scene, format!("{name}"));
                }
            }
            Class::DataSet | Class::Grant | Class::Bottle | Class::TimeSet | Class::NpcFlag | Class::WorldBool => {
                self.note(m, name, "effect not modeled");
            }
            Class::Gameplay => {
                *self.assumed.entry(format!("opaque action {name}")).or_insert(0) += 1;
                if verbose {
                    push(m, RK::Ev, format!("{ev} {name} skipped (engine)"));
                }
            }
            _ => {
                if verbose {
                    push(m, RK::Ev, format!("{ev} {name}"));
                }
            }
        }
        None
    }
}
