use crate::flags::hash_name;
use crate::flow_sim::db::{Db, Def, Tok, Ty};
use crate::flow_sim::state::{State, V};

pub fn truth(db: &Db, st: &State, h: u32, d: u8) -> Option<bool> {
    if d > 12 {
        return None;
    }
    let f: &Def = db.defs.get(&h)?;
    match f.ty {
        Ty::Bool => match db.get(st, h)? {
            V::B(b) => Some(b),
            _ => None,
        },
        Ty::Exp => eval(db, st, &f.exp, d + 1),
        _ => None,
    }
}

fn enum_idx(f: &Def, v: u32) -> Option<usize> {
    f.vals.iter().position(|n| hash_name(n) == v)
}

fn cmp_enum(db: &Db, st: &State, k: &Tok, c: &Tok) -> Option<bool> {
    if c.op != 2 {
        return None;
    }
    let f: &Def = db.defs.get(&k.a)?;
    let cur: u32 = match db.get(st, k.a)? {
        V::E(e) => e,
        _ => return None,
    };
    let (x, y): (usize, usize) = (enum_idx(f, cur)?, enum_idx(f, c.a)?);
    Some(match k.op {
        1 => x == y,
        11 => x < y,
        _ => x > y,
    })
}

fn cmp_int(db: &Db, st: &State, k: &Tok, c: &Tok) -> Option<bool> {
    if c.op != 16 {
        return None;
    }
    let x: i64 = db.int(st, k.a)?;
    let y: i64 = c.a as i32 as i64;
    Some(match k.op {
        15 => x == y,
        18 => x < y,
        _ => x > y,
    })
}

pub fn eval(db: &Db, st: &State, t: &[Tok], d: u8) -> Option<bool> {
    let mut s: Vec<bool> = Vec::new();
    let mut i: usize = 0;
    while i < t.len() {
        let k: &Tok = &t[i];
        match k.op {
            0 | 10 => s.push(truth(db, st, k.a, d)?),
            3 => {
                let b: bool = s.pop()?;
                let a: bool = s.pop()?;
                s.push(a && b);
            }
            4 => {
                let b: bool = s.pop()?;
                let a: bool = s.pop()?;
                s.push(a || b);
            }
            5 => {
                let a: bool = s.pop()?;
                s.push(!a);
            }
            1 | 11 | 12 => {
                i += 1;
                s.push(cmp_enum(db, st, k, t.get(i)?)?);
            }
            15 | 18 | 19 => {
                i += 1;
                s.push(cmp_int(db, st, k, t.get(i)?)?);
            }
            _ => return None,
        }
        i += 1;
    }
    if s.len() == 1 { s.pop() } else { None }
}

pub fn text(db: &Db, h: u32, d: u8) -> String {
    let Some(f) = db.defs.get(&h) else { return db.name(h) };
    if f.ty != Ty::Exp || d > 3 {
        return db.name(h);
    }
    let mut s: Vec<String> = Vec::new();
    let mut i: usize = 0;
    while i < f.exp.len() {
        let k: &Tok = &f.exp[i];
        match k.op {
            0 | 10 => s.push(text(db, k.a, d + 1)),
            3 | 4 => {
                let b: String = s.pop().unwrap_or_default();
                let a: String = s.pop().unwrap_or_default();
                s.push(format!("({a} {} {b})", if k.op == 3 { "and" } else { "or" }));
            }
            5 => {
                let a: String = s.pop().unwrap_or_default();
                s.push(format!("not {a}"));
            }
            1 | 11 | 12 | 15 | 18 | 19 => {
                i += 1;
                let c: Tok = f.exp.get(i).copied().unwrap_or(Tok { op: 0, a: 0, b: 0 });
                let rhs: String = if c.op == 2 { db.enum_name(k.a, c.a) } else { (c.a as i32).to_string() };
                let o: &str = match k.op {
                    1 | 15 => "==",
                    11 | 18 => "<",
                    _ => ">",
                };
                s.push(format!("{} {o} {rhs}", db.name(k.a)));
            }
            _ => s.push(format!("op{}", k.op)),
        }
        i += 1;
    }
    s.pop().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Db {
        let en: Vec<String> = ["NotReady", "Ready", "Step01", "Step02", "Complete"].iter().map(|s| s.to_string()).collect();
        let mut d: Db = Db::from_defs(vec![
            ("Step_Q", Def { ty: Ty::Enum, dv: V::E(hash_name("NotReady")), vals: en, exp: Vec::new() }),
            ("Flag", Def { ty: Ty::Bool, dv: V::B(false), vals: Vec::new(), exp: Vec::new() }),
            ("Count", Def { ty: Ty::Int, dv: V::I(2), vals: Vec::new(), exp: Vec::new() }),
        ]);
        let q: u32 = hash_name("Step_Q");
        let f: u32 = hash_name("Flag");
        let c: u32 = hash_name("Count");
        let after: Vec<Tok> = vec![Tok { op: 12, a: q, b: 0 }, Tok { op: 2, a: hash_name("Ready"), b: 0 }];
        let both: Vec<Tok> = vec![
            Tok { op: 10, a: hash_name("After"), b: 0 },
            Tok { op: 0, a: f, b: 0 },
            Tok { op: 5, a: 0, b: 0 },
            Tok { op: 3, a: 0, b: 0 },
        ];
        let cnt: Vec<Tok> = vec![Tok { op: 19, a: c, b: 0 }, Tok { op: 16, a: 1, b: 0 }];
        for (n, x) in [("After", after), ("Both", both), ("Cnt", cnt)] {
            let h: u32 = d.reg(n);
            d.defs.insert(h, Def { ty: Ty::Exp, dv: V::B(false), vals: Vec::new(), exp: x });
        }
        d
    }

    #[test]
    fn enum_and_int_comparisons_follow_the_token_pairs() {
        let d: Db = db();
        let mut s: State = State::new();
        assert_eq!(truth(&d, &s, hash_name("After"), 0), Some(false));
        s.v.insert(hash_name("Step_Q"), V::E(hash_name("Step02")));
        assert_eq!(truth(&d, &s, hash_name("After"), 0), Some(true));
        assert_eq!(truth(&d, &s, hash_name("Both"), 0), Some(true));
        s.v.insert(hash_name("Flag"), V::B(true));
        assert_eq!(truth(&d, &s, hash_name("Both"), 0), Some(false));
        assert_eq!(truth(&d, &s, hash_name("Cnt"), 0), Some(true));
        s.v.insert(hash_name("Count"), V::I(1));
        assert_eq!(truth(&d, &s, hash_name("Cnt"), 0), Some(false));
    }

    #[test]
    fn unknown_flags_give_no_answer() {
        let d: Db = db();
        assert_eq!(truth(&d, &State::new(), hash_name("Nope"), 0), None);
    }
}
