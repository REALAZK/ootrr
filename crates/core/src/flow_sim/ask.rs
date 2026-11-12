use crate::bfevfl::{Cont, Val};
use crate::flow_sim::expr;
use crate::flow_sim::run::{Env, Mach, Sim};
use crate::flow_sim::state::V;
use crate::flow_sim::vocab::{self, Class};

pub const RUPEE: &str = "PlayerStatus.CurrentRupee";
pub const CHILD: &str = "Sequence_IsChildhood";

pub struct Pr<'a> {
    pub env: &'a Env,
    pub c: &'a Option<Cont>,
}

impl<'a> Pr<'a> {
    pub fn val(&self, k: &str) -> Option<Val> {
        let v: &Val = self.c.as_ref()?.get(k)?;
        match v {
            Val::Arg(n) => self.env.get(n).cloned(),
            x => Some(x.clone()),
        }
    }

    pub fn s(&self, k: &str) -> Option<String> {
        match self.val(k)? {
            Val::Str(s) => Some(s),
            Val::Actor(a, _) => Some(a),
            _ => None,
        }
    }

    pub fn i(&self, k: &str) -> Option<i64> {
        match self.val(k)? {
            Val::Int(x) => Some(x as i64),
            Val::Bool(b) => Some((b != 0) as i64),
            Val::Float(f) => Some(f as i64),
            _ => None,
        }
    }

    pub fn b(&self, k: &str) -> Option<bool> {
        match self.val(k)? {
            Val::Bool(b) => Some(b != 0),
            Val::Int(x) => Some(x != 0),
            _ => None,
        }
    }
}

pub enum Sel {
    First,
    No,
}

pub enum Ans {
    One(u32, String),
    Any(String),
    Assume(Sel, String),
}

fn tf(b: bool) -> u32 {
    if b { 0 } else { 1 }
}

impl Sim {
    fn flag(&mut self, m: &Mach, n: &str) -> Result<bool, String> {
        let h: u32 = self.db.reg(n);
        if !self.db.defs.contains_key(&h) {
            self.unknown.insert(n.to_string());
            return Err(format!("unknown flag {n}"));
        }
        expr::truth(&self.db, &m.st, h, 0).ok_or_else(|| format!("{n} cannot be evaluated"))
    }

    fn int_of(&mut self, m: &Mach, n: &str) -> Result<i64, String> {
        let h: u32 = self.db.reg(n);
        if !self.db.defs.contains_key(&h) {
            self.unknown.insert(n.to_string());
            return Err(format!("unknown flag {n}"));
        }
        self.db.int(&m.st, h).ok_or_else(|| format!("{n} is not an int"))
    }

    fn stock(&mut self, m: &Mach, item: &str) -> i64 {
        let n: String = format!("PouchData.{item}.StockNum");
        let h: u32 = self.db.reg(&n);
        if self.db.defs.contains_key(&h) {
            return self.db.int(&m.st, h).unwrap_or(0);
        }
        m.st.grants.get(item).copied().unwrap_or(0)
    }

    fn data(&mut self, m: &Mach, q: &str, p: &Pr) -> Result<Ans, String> {
        let name: Option<String> = p.s("GameDataName");
        let need = |o: Option<String>| o.ok_or_else(|| format!("{q} has no resolvable name"));
        Ok(match q {
            "EventQueryIsGameDataBool" | "EventQueryIsGameDataBoolExp" => {
                let n: String = need(name)?;
                if p.i("Index").unwrap_or(-1) >= 0 {
                    return Err(format!("{n} is an array element"));
                }
                let b: bool = self.flag(m, &n)?;
                let h: u32 = self.db.reg(&n);
                let x: String = expr::text(&self.db, h, 0);
                let sh: String = if x == n { n } else { format!("{n} [{x}]") };
                Ans::One(tf(b), format!("{sh} = {b}"))
            }
            "EventQueryIsGameDataInt" => {
                let n: String = need(name)?;
                let v: i64 = p.i("Value").ok_or("Value is not resolvable")?;
                let c: i64 = self.int_of(m, &n)?;
                let r: u32 = if c < v { 0 } else if c == v { 1 } else { 2 };
                Ans::One(r, format!("{n} = {c} vs {v}"))
            }
            "EventQueryGameDataIntBranch" => {
                let n: String = need(name)?;
                let c: i64 = self.int_of(m, &n)?;
                Ans::One(c.max(0) as u32, format!("{n} = {c}"))
            }
            "EventQueryIsGameDataEnum" => {
                let n: String = need(name)?;
                let v: String = p.s("Value").ok_or("Value is not resolvable")?;
                let h: u32 = self.db.reg(&n);
                let cur: Option<V> = self.db.get(&m.st, h);
                let want: V = V::E(crate::flags::hash_name(&v));
                Ans::One(tf(cur.as_ref() == Some(&want)), format!("{n} == {v}"))
            }
            "EventQueryIsGameDataString64" => {
                let n: String = need(name)?;
                let v: String = p.s("Value").ok_or("Value is not resolvable")?;
                let h: u32 = self.db.reg(&n);
                let cur: Option<V> = self.db.get(&m.st, h);
                Ans::One(tf(cur == Some(V::S(v.clone()))), format!("{n} == {v:?}"))
            }
            "EventQueryIsBool" => {
                let b: bool = p.b("BoolValue").ok_or("BoolValue is not resolvable")?;
                Ans::One(tf(b), format!("argument = {b}"))
            }
            "EventQueryHavePlayedEvent" => {
                let e: String = p.s("EventName").ok_or("EventName is not resolvable")?;
                let n: String = format!("HavePlayedEvent.{e}");
                let b: bool = self.flag(m, &n)?;
                Ans::One(tf(b), format!("{n} = {b}"))
            }
            "EventQueryCheckIsGet" => {
                let a: String = p.s("ActorName").ok_or("ActorName is not resolvable")?;
                let n: String = format!("IsGet.{a}");
                let b: bool = self.flag(m, &n)?;
                Ans::One(tf(b), format!("{n} = {b}"))
            }
            _ => return Err(format!("{q} is not modeled")),
        })
    }

    fn pouch(&mut self, m: &Mach, q: &str, p: &Pr) -> Result<Ans, String> {
        Ok(match q {
            "EventQueryIsExistInPouch" => {
                let a: String = p.s("Actor").ok_or("Actor is not resolvable")?;
                let n: i64 = p.i("Num").unwrap_or(1);
                let c: i64 = self.stock(m, &a);
                Ans::One(tf(c >= n), format!("stock of {a} = {c}, needs {n}"))
            }
            "EventQueryHasRuppe" => {
                let v: i64 = p.i("Value").ok_or("Value is not resolvable")?;
                let c: i64 = self.int_of(m, RUPEE)?;
                Ans::One(tf(c >= v), format!("rupees {c}, needs {v}"))
            }
            "EventQueryHeartPieceNum" => {
                let c: i64 = self.stock(m, "Obj_Heart_Piece");
                Ans::One((c.max(0) as u32).min(3), format!("heart pieces {c}"))
            }
            _ => return Err(format!("{q} is not modeled")),
        })
    }

    fn ask(&mut self, m: &mut Mach, actor: &str, q: &str, p: &Pr, cls: Class) -> Ans {
        let r: Result<Ans, String> = match cls {
            Class::QData => self.data(m, q, p),
            Class::QPouch => self.pouch(m, q, p),
            _ => Err(String::new()),
        };
        match r {
            Ok(a) => return a,
            Err(e) if !e.is_empty() => return Ans::Any(e),
            Err(_) => {}
        }
        match cls {
            Class::QChoice => {
                if m.chosen < self.opts.choose.len() {
                    let v: u32 = self.opts.choose[m.chosen];
                    m.chosen += 1;
                    return Ans::One(v, format!("player picks option {v}"));
                }
                Ans::Any("player choice".to_string())
            }
            Class::QRandom => Ans::Any("random".to_string()),
            Class::QTime => match (q, self.opts.hour) {
                ("EventQueryIsCurrentTimeInSpecifiedTimeRange", Some(h)) => {
                    let s: i64 = p.i("StartTime").unwrap_or(0);
                    let e: i64 = p.i("EndTme").or(p.i("EndTime")).unwrap_or(0);
                    let t: i64 = h as i64;
                    let inside: bool = if s <= e { t >= s && t < e } else { t >= s || t < e };
                    Ans::One(tf(inside), format!("hour {h} in {s}..{e}"))
                }
                _ => Ans::Any("time of day".to_string()),
            },
            Class::QNpc => {
                let kind: &str = match q {
                    "EventQueryIsOnRecentTalkFlag" => "recent",
                    "EventQueryIsOnMaskTalkFlag" | "EventQueryIsOnAmiiboMaskTalkFlag" => "mask",
                    "EventQueryVolatileGeneralPurposeFlagOn" => "volatile",
                    _ => "talked",
                };
                match m.st.npc.get(&format!("{actor}.{kind}")) {
                    Some(true) => Ans::One(0, format!("{actor} {kind} flag is set")),
                    _ => Ans::Assume(Sel::No, format!("{actor} {kind} flag assumed clear")),
                }
            }
            Class::QPlayer if q == "EventQueryPlayerIsChild" => match self.flag(m, CHILD) {
                Ok(b) => Ans::One(tf(b), format!("{CHILD} = {b}")),
                Err(e) => Ans::Any(e),
            },
            Class::QPlayer => Ans::Assume(Sel::No, "player state assumed false".to_string()),
            Class::QScene => Ans::Assume(Sel::First, "scene or AI state assumed first case".to_string()),
            _ => Ans::Any("engine decides".to_string()),
        }
    }

    pub fn pick(&mut self, m: &mut Mach, actor: &str, q: &str, c: &Option<Cont>, env: &Env, vals: &[u32]) -> Vec<(u32, String, bool)> {
        let p: Pr = Pr { env, c };
        if let Some(v) = self.opts.answers.get(q) {
            return vec![(*v, format!("scripted {v}"), false)];
        }
        let cls: Class = vocab::query(q);
        let a: Ans = self.ask(m, actor, q, &p, cls);
        match a {
            Ans::One(v, w) => vec![(v, w, false)],
            Ans::Any(w) => {
                *self.assumed.entry(format!("branch on {q}")).or_insert(0) += 1;
                vals.iter().map(|v| (*v, format!("{w} case {v}"), false)).collect()
            }
            Ans::Assume(s, w) => {
                *self.assumed.entry(format!("assume {q}")).or_insert(0) += 1;
                let v: Option<u32> = match s {
                    Sel::First => vals.first().copied(),
                    Sel::No => vals.iter().copied().find(|x| *x == 1).or(vals.first().copied()),
                };
                match v {
                    Some(v) => vec![(v, format!("{w} ({v})"), true)],
                    None => Vec::new(),
                }
            }
        }
    }
}
