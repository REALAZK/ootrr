use std::collections::HashMap as HM;

use crate::error::{Error as E, Res};
use crate::logic_items::{ItemId, Items};
use crate::logic_json::Json;
use crate::logic_val::{same, Settings, Val};
use crate::rule_ast::{parse_rule, Ast, Bop, Cmp};

pub const DAY: u8 = 1;
pub const DAMPE: u8 = 2;
pub const ALL_TIME: u8 = 3;
const STEP_LIMIT: usize = 400_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Age {
    Child,
    Adult,
}

impl Age {
    pub const BOTH: [Age; 2] = [Age::Adult, Age::Child];

    pub fn idx(self) -> usize {
        match self {
            Age::Adult => 0,
            Age::Child => 1,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Age::Adult => "adult",
            Age::Child => "child",
        }
    }

    pub fn from_name(s: &str) -> Option<Age> {
        match s {
            "adult" => Some(Age::Adult),
            "child" => Some(Age::Child),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Node {
    Const(Val),
    Has(ItemId, u32),
    Any(Vec<ItemId>),
    All(Vec<ItemId>),
    Item(ItemId),
    Distinct(Vec<ItemId>),
    Age,
    AgeIs(Age),
    Tod(u8, Box<Node>),
    Not(Box<Node>),
    Neg(Box<Node>),
    And(Vec<Node>),
    Or(Vec<Node>),
    Cmp(Cmp, Box<Node>, Box<Node>),
    Bin(Bop, Box<Node>, Box<Node>),
    If(Box<Node>, Box<Node>, Box<Node>),
    List(Vec<Node>),
}

#[derive(Clone, Debug)]
pub struct Rule {
    pub node: Node,
    pub deps: Vec<ItemId>,
    pub tod: bool,
    pub always: bool,
    pub never: bool,
}

impl Rule {
    pub fn new(node: Node) -> Rule {
        let mut deps: Vec<ItemId> = Vec::new();
        let mut tod: bool = false;
        collect_deps(&node, &mut deps, &mut tod);
        deps.sort_unstable();
        deps.dedup();
        let (always, never): (bool, bool) = match &node {
            Node::Const(v) => (v.truthy(), !v.truthy()),
            _ => (false, false),
        };
        Rule { node, deps, tod, always, never }
    }

    pub fn truth(b: bool) -> Rule {
        Rule::new(Node::Const(Val::Bool(b)))
    }
}

fn collect_deps(n: &Node, out: &mut Vec<ItemId>, tod: &mut bool) {
    match n {
        Node::Const(_) | Node::Age | Node::AgeIs(_) => {}
        Node::Has(i, _) | Node::Item(i) => out.push(*i),
        Node::Any(v) | Node::All(v) | Node::Distinct(v) => out.extend(v.iter().copied()),
        Node::Tod(_, s) => {
            *tod = true;
            collect_deps(s, out, tod);
        }
        Node::Not(x) | Node::Neg(x) => collect_deps(x, out, tod),
        Node::And(v) | Node::Or(v) | Node::List(v) => {
            for x in v {
                collect_deps(x, out, tod);
            }
        }
        Node::Cmp(_, a, b) | Node::Bin(_, a, b) => {
            collect_deps(a, out, tod);
            collect_deps(b, out, tod);
        }
        Node::If(a, b, c) => {
            collect_deps(a, out, tod);
            collect_deps(b, out, tod);
            collect_deps(c, out, tod);
        }
    }
}

pub trait Reach {
    fn reach_tod(&self, region: usize, age: Age, flag: u8) -> bool;
}

pub struct Ctx<'a> {
    pub cnt: &'a [u32],
    pub age: Option<Age>,
    pub tod: u8,
    pub region: usize,
    pub q: Option<&'a dyn Reach>,
}

fn floor_div(a: i64, b: i64) -> Option<i64> {
    if b == 0 {
        return None;
    }
    let q: i64 = a.wrapping_div(b);
    if a.wrapping_rem(b) != 0 && ((a < 0) != (b < 0)) {
        Some(q - 1)
    } else {
        Some(q)
    }
}

pub fn arith(op: Bop, a: &Val, b: &Val) -> Option<Val> {
    if let (Val::Int(x), Val::Int(y)) = (a, b) {
        return match op {
            Bop::Add => Some(Val::Int(x.wrapping_add(*y))),
            Bop::Sub => Some(Val::Int(x.wrapping_sub(*y))),
            Bop::Mul => Some(Val::Int(x.wrapping_mul(*y))),
            Bop::Floor => floor_div(*x, *y).map(Val::Int),
            Bop::Rem => {
                let q: i64 = floor_div(*x, *y)?;
                Some(Val::Int(x.wrapping_sub(q.wrapping_mul(*y))))
            }
            Bop::Div => {
                if *y == 0 {
                    None
                } else {
                    Some(Val::Float(*x as f64 / *y as f64))
                }
            }
        };
    }
    if let (Val::Str(x), Val::Str(y)) = (a, b)
        && op == Bop::Add
    {
        return Some(Val::Str(format!("{x}{y}")));
    }
    let x: f64 = a.num()?;
    let y: f64 = b.num()?;
    match op {
        Bop::Add => Some(Val::Float(x + y)),
        Bop::Sub => Some(Val::Float(x - y)),
        Bop::Mul => Some(Val::Float(x * y)),
        Bop::Div => {
            if y == 0.0 {
                None
            } else {
                Some(Val::Float(x / y))
            }
        }
        Bop::Floor => {
            if y == 0.0 {
                None
            } else {
                Some(Val::Float((x / y).floor()))
            }
        }
        Bop::Rem => {
            if y == 0.0 {
                None
            } else {
                Some(Val::Float(x - y * (x / y).floor()))
            }
        }
    }
}

pub fn compare(op: Cmp, a: &Val, b: &Val) -> bool {
    match op {
        Cmp::Eq => same(a, b),
        Cmp::Ne => !same(a, b),
        Cmp::In => member(a, b),
        Cmp::NotIn => !member(a, b),
        Cmp::Lt | Cmp::Le | Cmp::Gt | Cmp::Ge => {
            let o: Option<std::cmp::Ordering> = match (a.num(), b.num()) {
                (Some(x), Some(y)) => x.partial_cmp(&y),
                _ => match (a, b) {
                    (Val::Str(x), Val::Str(y)) => Some(x.cmp(y)),
                    _ => None,
                },
            };
            match o {
                None => false,
                Some(o) => match op {
                    Cmp::Lt => o.is_lt(),
                    Cmp::Le => o.is_le(),
                    Cmp::Gt => o.is_gt(),
                    _ => o.is_ge(),
                },
            }
        }
    }
}

fn member(a: &Val, b: &Val) -> bool {
    match b {
        Val::List(v) => v.iter().any(|x| same(a, x)),
        Val::Str(s) => matches!(a, Val::Str(t) if s.contains(t.as_str())),
        Val::Map(m) => matches!(a, Val::Str(t) if m.contains_key(t)),
        _ => false,
    }
}

pub fn eval(n: &Node, c: &Ctx) -> bool {
    match n {
        Node::Const(v) => v.truthy(),
        Node::Has(i, k) => c.cnt[*i] >= *k,
        Node::Any(v) => v.iter().any(|i| c.cnt[*i] > 0),
        Node::All(v) => v.iter().all(|i| c.cnt[*i] > 0),
        Node::AgeIs(a) => c.age == Some(*a),
        Node::Tod(f, skip) => {
            if c.tod != 0 {
                return c.tod & *f != 0;
            }
            if eval(skip, c) {
                return true;
            }
            match c.q {
                None => false,
                Some(q) => match c.age {
                    Some(a) => q.reach_tod(c.region, a, *f),
                    None => Age::BOTH.iter().any(|a| q.reach_tod(c.region, *a, *f)),
                },
            }
        }
        Node::Not(x) => !eval(x, c),
        Node::And(v) => v.iter().all(|x| eval(x, c)),
        Node::Or(v) => v.iter().any(|x| eval(x, c)),
        _ => value(n, c).truthy(),
    }
}

pub fn value(n: &Node, c: &Ctx) -> Val {
    match n {
        Node::Const(v) => v.clone(),
        Node::Item(i) => Val::Int(c.cnt[*i] as i64),
        Node::Distinct(v) => Val::Int(v.iter().filter(|i| c.cnt[**i] > 0).count() as i64),
        Node::Age => match c.age {
            Some(a) => Val::Str(a.name().to_string()),
            None => Val::Bool(false),
        },
        Node::Neg(x) => match value(x, c) {
            Val::Int(k) => Val::Int(k.wrapping_neg()),
            Val::Float(f) => Val::Float(-f),
            _ => Val::Bool(false),
        },
        Node::Bin(op, a, b) => arith(*op, &value(a, c), &value(b, c)).unwrap_or(Val::Bool(false)),
        Node::Cmp(op, a, b) => Val::Bool(compare(*op, &value(a, c), &value(b, c))),
        Node::If(k, t, e) => {
            if eval(k, c) {
                value(t, c)
            } else {
                value(e, c)
            }
        }
        Node::List(v) => Val::List(v.iter().map(|x| value(x, c)).collect()),
        _ => Val::Bool(eval(n, c)),
    }
}

#[derive(Clone, Debug)]
pub struct Alias {
    pub params: Vec<String>,
    pub body: Ast,
}

#[derive(Clone, Debug, Default)]
pub struct Aliases {
    pub map: HM<String, Alias>,
}

impl Aliases {
    pub fn new() -> Aliases {
        Aliases { map: HM::new() }
    }

    pub fn load(&mut self, j: &Json) -> Res<()> {
        let o: &[(String, Json)] = j.as_obj().ok_or(E::Format("helpers must be an object"))?;
        for (k, v) in o {
            let text: &str = v.as_str().ok_or_else(|| E::Msg(format!("helper {k} must be a string")))?;
            let (name, params): (String, Vec<String>) = match k.split_once('(') {
                Some((a, b)) => {
                    let inner: &str = b.trim_end().trim_end_matches(')');
                    let ps: Vec<String> = inner
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    (a.trim().to_string(), ps)
                }
                None => (k.trim().to_string(), Vec::new()),
            };
            let body: Ast = parse_rule(text).map_err(|e| E::Msg(format!("helper {k}: {e}")))?;
            if self.map.insert(name.clone(), Alias { params, body }).is_some() {
                return Err(E::Msg(format!("duplicate helper {name}")));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Spot {
    pub region: String,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct Sub {
    pub region: String,
    pub ast: Ast,
    pub item: ItemId,
    pub name: String,
}

fn subst(a: &Ast, m: &[(String, Ast)]) -> Ast {
    match a {
        Ast::Name(n) => match m.iter().find(|(k, _)| k == n) {
            Some((_, v)) => v.clone(),
            None => a.clone(),
        },
        Ast::Tuple(v) => Ast::Tuple(v.iter().map(|x| subst(x, m)).collect()),
        Ast::List(v) => Ast::List(v.iter().map(|x| subst(x, m)).collect()),
        Ast::And(v) => Ast::And(v.iter().map(|x| subst(x, m)).collect()),
        Ast::Or(v) => Ast::Or(v.iter().map(|x| subst(x, m)).collect()),
        Ast::Call(n, v) => Ast::Call(n.clone(), v.iter().map(|x| subst(x, m)).collect()),
        Ast::Index(n, x) => Ast::Index(n.clone(), Box::new(subst(x, m))),
        Ast::Not(x) => Ast::Not(Box::new(subst(x, m))),
        Ast::Neg(x) => Ast::Neg(Box::new(subst(x, m))),
        Ast::Cmp(l, r) => Ast::Cmp(
            Box::new(subst(l, m)),
            r.iter().map(|(o, x)| (*o, subst(x, m))).collect(),
        ),
        Ast::Bin(o, x, y) => Ast::Bin(*o, Box::new(subst(x, m)), Box::new(subst(y, m))),
        Ast::If(c, t, e) => Ast::If(Box::new(subst(c, m)), Box::new(subst(t, m)), Box::new(subst(e, m))),
        Ast::Str(_) | Ast::Int(_) | Ast::Float(_) | Ast::Bool(_) => a.clone(),
    }
}

fn event_like(n: &str) -> bool {
    n.chars().next().is_some_and(|c| c.is_uppercase())
}

fn push_unique(v: &mut Vec<ItemId>, i: ItemId) {
    if !v.contains(&i) {
        v.push(i);
    }
}

pub struct Compiler<'a> {
    pub items: &'a mut Items,
    pub settings: &'a Settings,
    pub aliases: &'a Aliases,
    pub groups: &'a HM<String, Vec<ItemId>>,
    pub lenient: bool,
    pub subs: Vec<Sub>,
    sub_ix: HM<(String, String), ItemId>,
    sub_n: HM<String, usize>,
    steps: usize,
}

impl<'a> Compiler<'a> {
    pub fn new(
        items: &'a mut Items,
        settings: &'a Settings,
        aliases: &'a Aliases,
        groups: &'a HM<String, Vec<ItemId>>,
        lenient: bool,
    ) -> Compiler<'a> {
        Compiler {
            items,
            settings,
            aliases,
            groups,
            lenient,
            subs: Vec::new(),
            sub_ix: HM::new(),
            sub_n: HM::new(),
            steps: 0,
        }
    }

    pub fn rule(&mut self, text: &str, sp: &Spot) -> Res<Rule> {
        let a: Ast = parse_rule(text).map_err(|e| E::Msg(format!("{}: {e}", sp.name)))?;
        self.rule_ast(&a, sp)
    }

    pub fn rule_ast(&mut self, a: &Ast, sp: &Spot) -> Res<Rule> {
        self.steps = 0;
        let n: Node = self.comp(a, sp, 0).map_err(|e| E::Msg(format!("{}: {e}", sp.name)))?;
        Ok(Rule::new(n))
    }

    fn fail<T>(&self, m: String) -> Res<T> {
        Err(E::Msg(m))
    }

    fn lenient_or<T>(&self, v: T, m: String) -> Res<T> {
        if self.lenient { Ok(v) } else { self.fail(m) }
    }

    fn tick(&mut self, d: usize) -> Res<()> {
        self.steps += 1;
        if d > 64 || self.steps > STEP_LIMIT {
            return self.fail("expansion too deep or too large".to_string());
        }
        Ok(())
    }

    fn comp(&mut self, a: &Ast, sp: &Spot, d: usize) -> Res<Node> {
        self.tick(d)?;
        match a {
            Ast::Bool(b) => Ok(Node::Const(Val::Bool(*b))),
            Ast::Int(n) => Ok(Node::Const(Val::Int(*n))),
            Ast::Float(f) => Ok(Node::Const(Val::Float(*f))),
            Ast::Str(s) => {
                let i: ItemId = self.items.event_named(s);
                Ok(Node::Has(i, 1))
            }
            Ast::Name(n) => self.name(n, sp, d),
            Ast::Tuple(v) => self.tuple(v, sp, d),
            Ast::List(_) => self.opnd(a, sp, d),
            Ast::Call(n, v) => self.call(n, v, sp, d),
            Ast::Index(n, x) => self.index(n, x, sp, d),
            Ast::Not(x) => {
                let n: Node = self.comp(x, sp, d + 1)?;
                match n {
                    Node::Const(v) => Ok(Node::Const(Val::Bool(!v.truthy()))),
                    o => Ok(Node::Not(Box::new(o))),
                }
            }
            Ast::Neg(_) | Ast::Bin(_, _, _) => self.opnd(a, sp, d),
            Ast::And(v) => self.boolop(false, v, sp, d),
            Ast::Or(v) => self.boolop(true, v, sp, d),
            Ast::Cmp(l, r) => self.cmp(l, r, sp, d),
            Ast::If(c, t, e) => {
                let cn: Node = self.comp(c, sp, d + 1)?;
                if let Node::Const(v) = &cn {
                    return if v.truthy() { self.comp(t, sp, d + 1) } else { self.comp(e, sp, d + 1) };
                }
                let tn: Node = self.comp(t, sp, d + 1)?;
                let en: Node = self.comp(e, sp, d + 1)?;
                Ok(Node::If(Box::new(cn), Box::new(tn), Box::new(en)))
            }
        }
    }

    fn known(&self, n: &str) -> bool {
        self.settings.get(n).is_some()
            || self.aliases.map.contains_key(n)
            || matches!(n, "age" | "at_day" | "at_night" | "at_dampe_time")
            || event_like(n)
    }

    fn opnd(&mut self, a: &Ast, sp: &Spot, d: usize) -> Res<Node> {
        self.tick(d)?;
        match a {
            Ast::Str(s) => Ok(Node::Const(Val::Str(s.clone()))),
            Ast::Name(n) => {
                if let Some(i) = self.items.find_esc(n)
                    && self.settings.get(n).is_none()
                {
                    return Ok(Node::Const(Val::Str(self.items.names[i].clone())));
                }
                if self.known(n) {
                    self.comp(a, sp, d + 1)
                } else {
                    Ok(Node::Const(Val::Str(n.clone())))
                }
            }
            Ast::Tuple(v) | Ast::List(v) => {
                let mut out: Vec<Node> = Vec::with_capacity(v.len());
                for x in v {
                    out.push(self.opnd(x, sp, d + 1)?);
                }
                if out.iter().all(|x| matches!(x, Node::Const(_))) {
                    let vals: Vec<Val> = out
                        .into_iter()
                        .filter_map(|x| match x {
                            Node::Const(v) => Some(v),
                            _ => None,
                        })
                        .collect();
                    Ok(Node::Const(Val::List(vals)))
                } else {
                    Ok(Node::List(out))
                }
            }
            Ast::Neg(x) => {
                let n: Node = self.opnd(x, sp, d + 1)?;
                match n {
                    Node::Const(Val::Int(k)) => Ok(Node::Const(Val::Int(k.wrapping_neg()))),
                    Node::Const(Val::Float(f)) => Ok(Node::Const(Val::Float(-f))),
                    Node::Const(_) => self.fail("cannot negate a non-number".to_string()),
                    o => Ok(Node::Neg(Box::new(o))),
                }
            }
            Ast::Bin(op, x, y) => {
                let l: Node = self.opnd(x, sp, d + 1)?;
                let r: Node = self.opnd(y, sp, d + 1)?;
                if let (Node::Const(p), Node::Const(q)) = (&l, &r) {
                    return match arith(*op, p, q) {
                        Some(v) => Ok(Node::Const(v)),
                        None => self.fail("invalid arithmetic".to_string()),
                    };
                }
                Ok(Node::Bin(*op, Box::new(l), Box::new(r)))
            }
            _ => self.comp(a, sp, d + 1),
        }
    }

    fn name(&mut self, n: &str, sp: &Spot, d: usize) -> Res<Node> {
        match n {
            "at_day" | "at_night" | "at_dampe_time" => return self.tod(n, sp, d),
            "age" => return Ok(Node::Age),
            _ => {}
        }
        if let Some(al) = self.aliases.map.get(n) {
            if !al.params.is_empty() {
                let m: String = format!("expected {} args for {n}, not 0", al.params.len());
                return self.fail(m);
            }
            let body: Ast = al.body.clone();
            return self.comp(&body, sp, d + 1);
        }
        if let Some(i) = self.items.find_esc(n) {
            return Ok(Node::Has(i, 1));
        }
        if let Some(v) = self.settings.get(n) {
            return Ok(Node::Const(v.clone()));
        }
        if event_like(n) {
            let i: ItemId = self.items.event_named(&n.replace('_', " "));
            return Ok(Node::Has(i, 1));
        }
        self.lenient_or(Node::Const(Val::Bool(false)), format!("invalid name {n}"))
    }

    fn tod(&mut self, w: &str, sp: &Spot, d: usize) -> Res<Node> {
        if !self.settings.flag("ensure_tod_access") {
            return Ok(Node::Const(Val::Bool(true)));
        }
        let (flag, skip): (u8, &str) = match w {
            "at_day" => (DAY, "skip_to_day"),
            "at_night" => (DAMPE, "skip_to_night"),
            _ => (DAMPE, "skip_to_dampe"),
        };
        let sn: Node = if self.aliases.map.contains_key(skip) {
            self.comp(&Ast::Name(skip.to_string()), sp, d + 1)?
        } else {
            Node::Const(Val::Bool(false))
        };
        Ok(Node::Tod(flag, Box::new(sn)))
    }

    fn item_arg(&mut self, a: &Ast) -> Res<ItemId> {
        match a {
            Ast::Str(s) => Ok(self.items.event_named(s)),
            Ast::Name(n) => {
                if let Some(i) = self.items.find_esc(n) {
                    Ok(i)
                } else if event_like(n) {
                    Ok(self.items.event_named(&n.replace('_', " ")))
                } else {
                    self.fail(format!("invalid item {n}"))
                }
            }
            _ => self.fail("expected an item".to_string()),
        }
    }

    fn items_arg(&mut self, v: &[Ast]) -> Res<Vec<ItemId>> {
        let flat: Vec<Ast> = match v {
            [Ast::Tuple(w)] | [Ast::List(w)] => w.clone(),
            _ => v.to_vec(),
        };
        let mut out: Vec<ItemId> = Vec::with_capacity(flat.len());
        for x in &flat {
            out.push(self.item_arg(x)?);
        }
        Ok(out)
    }

    fn const_int(&mut self, a: &Ast, sp: &Spot, d: usize) -> Res<i64> {
        match self.opnd(a, sp, d + 1)? {
            Node::Const(Val::Int(n)) => Ok(n),
            Node::Const(Val::Float(f)) if f.fract() == 0.0 => Ok(f as i64),
            _ => self.fail("expected a number".to_string()),
        }
    }

    fn tuple(&mut self, v: &[Ast], sp: &Spot, d: usize) -> Res<Node> {
        if v.len() != 2 {
            return self.fail("tuple must have 2 values".to_string());
        }
        let i: ItemId = self.item_arg(&v[0])?;
        let k: i64 = match self.const_int(&v[1], sp, d) {
            Ok(k) => k,
            Err(_) if self.lenient => return Ok(Node::Const(Val::Bool(false))),
            Err(e) => return Err(e),
        };
        if k <= 0 {
            return Ok(Node::Const(Val::Bool(true)));
        }
        Ok(Node::Has(i, k.min(u32::MAX as i64) as u32))
    }

    fn at(&mut self, region: &str, body: &Ast) -> Res<Node> {
        let key: (String, String) = (region.to_string(), format!("{body:?}"));
        if let Some(i) = self.sub_ix.get(&key) {
            return Ok(Node::Has(*i, 1));
        }
        let n: usize = self.sub_n.get(region).copied().unwrap_or(0) + 1;
        self.sub_n.insert(region.to_string(), n);
        let name: String = format!("{region} Subrule {n}");
        let item: ItemId = self.items.event_named(&name);
        self.sub_ix.insert(key, item);
        self.subs.push(Sub { region: region.to_string(), ast: body.clone(), item, name });
        Ok(Node::Has(item, 1))
    }

    fn call(&mut self, n: &str, v: &[Ast], sp: &Spot, d: usize) -> Res<Node> {
        match n {
            "at" => {
                return match v {
                    [Ast::Str(r), body, ..] => self.at(r, body),
                    _ => self.fail("invalid at() arguments".to_string()),
                };
            }
            "here" => {
                return match v.first() {
                    Some(body) => {
                        let r: String = sp.region.clone();
                        self.at(&r, body)
                    }
                    None => self.fail("missing here() argument".to_string()),
                };
            }
            "at_day" | "at_night" | "at_dampe_time" => return self.tod(n, sp, d),
            _ => {}
        }
        if let Some(al) = self.aliases.map.get(n) {
            if al.params.len() != v.len() {
                let m: String = format!("expected {} args for {n}, not {}", al.params.len(), v.len());
                return self.fail(m);
            }
            let mut m: Vec<(String, Ast)> = Vec::with_capacity(v.len());
            for (p, x) in al.params.iter().zip(v.iter()) {
                match x {
                    Ast::Name(_) | Ast::Str(_) | Ast::Int(_) | Ast::Float(_) | Ast::Bool(_) => {
                        m.push((p.clone(), x.clone()));
                    }
                    _ => return self.fail(format!("invalid argument for {n}")),
                }
            }
            let body: Ast = subst(&al.body, &m);
            return self.comp(&body, sp, d + 1);
        }
        match n {
            "has" => {
                let first: &Ast = v.first().ok_or(E::Format("has() needs an item"))?;
                let i: ItemId = self.item_arg(first)?;
                let k: i64 = match v.get(1) {
                    Some(x) => self.const_int(x, sp, d)?,
                    None => 1,
                };
                if k <= 0 {
                    return Ok(Node::Const(Val::Bool(true)));
                }
                Ok(Node::Has(i, k.min(u32::MAX as i64) as u32))
            }
            "has_any_of" => {
                let ids: Vec<ItemId> = self.items_arg(v)?;
                Ok(Node::Any(ids))
            }
            "has_all_of" => {
                let ids: Vec<ItemId> = self.items_arg(v)?;
                Ok(Node::All(ids))
            }
            "item_count" | "count" => {
                let first: &Ast = v.first().ok_or(E::Format("item_count() needs an item"))?;
                Ok(Node::Item(self.item_arg(first)?))
            }
            "count_distinct" => {
                let ids: Vec<ItemId> = self.items_arg(v)?;
                Ok(Node::Distinct(ids))
            }
            "count_group" | "has_group" => {
                let g: String = match v.first() {
                    Some(Ast::Name(s)) | Some(Ast::Str(s)) => s.clone(),
                    _ => return self.fail("group name expected".to_string()),
                };
                let ids: Vec<ItemId> = match self.groups.get(&g) {
                    Some(x) => x.clone(),
                    None => return self.lenient_or(Vec::new(), format!("unknown group {g}")).map(Node::Distinct),
                };
                if n == "count_group" {
                    return Ok(Node::Distinct(ids));
                }
                let k: i64 = match v.get(1) {
                    Some(x) => self.const_int(x, sp, d)?,
                    None => 1,
                };
                Ok(Node::Cmp(
                    Cmp::Ge,
                    Box::new(Node::Distinct(ids)),
                    Box::new(Node::Const(Val::Int(k))),
                ))
            }
            _ => self.lenient_or(Node::Const(Val::Bool(false)), format!("unknown function {n}")),
        }
    }

    fn index(&mut self, n: &str, x: &Ast, sp: &Spot, d: usize) -> Res<Node> {
        let key: String = match x {
            Ast::Name(k) => k.replace('_', " "),
            Ast::Str(k) => k.clone(),
            o => match self.opnd(o, sp, d + 1)? {
                Node::Const(Val::Str(s)) => s,
                _ => return self.fail("unsupported subscript".to_string()),
            },
        };
        match self.settings.get(n) {
            Some(Val::Map(m)) => match m.get(&key) {
                Some(v) => Ok(Node::Const(v.clone())),
                None => self.lenient_or(Node::Const(Val::Bool(false)), format!("no key {key} in {n}")),
            },
            _ => self.lenient_or(Node::Const(Val::Bool(false)), format!("{n} is not a table")),
        }
    }

    fn boolop(&mut self, is_or: bool, v: &[Ast], sp: &Spot, d: usize) -> Res<Node> {
        let mut group: Vec<ItemId> = Vec::new();
        let mut rest: Vec<Node> = Vec::new();
        let mut stack: Vec<Node> = Vec::with_capacity(v.len());
        for x in v {
            stack.push(self.comp(x, sp, d + 1)?);
        }
        let mut k: usize = 0;
        while k < stack.len() {
            let n: Node = std::mem::replace(&mut stack[k], Node::Const(Val::Bool(true)));
            k += 1;
            match n {
                Node::Const(c) => {
                    if c.truthy() == is_or {
                        return Ok(Node::Const(Val::Bool(is_or)));
                    }
                }
                Node::Has(i, 1) => push_unique(&mut group, i),
                Node::Any(w) if is_or => {
                    for i in w {
                        push_unique(&mut group, i);
                    }
                }
                Node::All(w) if !is_or => {
                    for i in w {
                        push_unique(&mut group, i);
                    }
                }
                Node::Or(w) if is_or => rest.extend(w),
                Node::And(w) if !is_or => rest.extend(w),
                o => rest.push(o),
            }
        }
        let mut out: Vec<Node> = Vec::new();
        match group.len() {
            0 => {}
            1 => out.push(Node::Has(group[0], 1)),
            _ => out.push(if is_or { Node::Any(group) } else { Node::All(group) }),
        }
        out.extend(rest);
        match out.len() {
            0 => Ok(Node::Const(Val::Bool(!is_or))),
            1 => Ok(out.remove(0)),
            _ => Ok(if is_or { Node::Or(out) } else { Node::And(out) }),
        }
    }

    fn pair(&mut self, op: Cmp, l: Node, r: Node) -> Node {
        if let (Node::Const(a), Node::Const(b)) = (&l, &r) {
            return Node::Const(Val::Bool(compare(op, a, b)));
        }
        if op == Cmp::Eq || op == Cmp::Ne {
            let side: Option<Age> = match (&l, &r) {
                (Node::Age, Node::Const(Val::Str(s))) | (Node::Const(Val::Str(s)), Node::Age) => Age::from_name(s),
                _ => None,
            };
            if let Some(a) = side {
                let t: Node = Node::AgeIs(a);
                return if op == Cmp::Eq { t } else { Node::Not(Box::new(t)) };
            }
        }
        Node::Cmp(op, Box::new(l), Box::new(r))
    }

    fn cmp(&mut self, l: &Ast, r: &[(Cmp, Ast)], sp: &Spot, d: usize) -> Res<Node> {
        if r.len() == 1
            && r[0].0 == Cmp::Eq
            && let (Ast::Name(a), Ast::Name(b)) = (l, &r[0].1)
            && self.settings.get(a).is_none()
            && self.settings.get(b).is_none()
        {
            return Ok(Node::Const(Val::Bool(a == b)));
        }
        let mut prev: Node = self.opnd(l, sp, d + 1)?;
        let mut parts: Vec<Node> = Vec::with_capacity(r.len());
        for (op, x) in r {
            let nx: Node = self.opnd(x, sp, d + 1)?;
            let p: Node = self.pair(*op, prev, nx.clone());
            parts.push(p);
            prev = nx;
        }
        if parts.len() == 1 {
            return Ok(parts.remove(0));
        }
        let mut all: Vec<Node> = Vec::new();
        for p in parts {
            match p {
                Node::Const(v) => {
                    if !v.truthy() {
                        return Ok(Node::Const(Val::Bool(false)));
                    }
                }
                o => all.push(o),
            }
        }
        match all.len() {
            0 => Ok(Node::Const(Val::Bool(true))),
            1 => Ok(all.remove(0)),
            _ => Ok(Node::And(all)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic_json::parse;

    struct Fx {
        items: Items,
        settings: Settings,
        aliases: Aliases,
        groups: HM<String, Vec<ItemId>>,
    }

    fn fx(settings: &[(&str, Val)], helpers: &str) -> Fx {
        let mut items: Items = Items::new();
        for n in ["Sword", "Bow", "Hookshot Piece", "Key", "Bomb Bag", "Dins Fire", "Ocarina"] {
            items.add(n, true, false).unwrap();
        }
        let mut s: Settings = Settings::new();
        for (k, v) in settings {
            s.set(k, v.clone());
        }
        let mut aliases: Aliases = Aliases::new();
        aliases.load(&parse(helpers).unwrap()).unwrap();
        Fx { items, settings: s, aliases, groups: HM::new() }
    }

    fn sp() -> Spot {
        Spot { region: "R".to_string(), name: "test".to_string() }
    }

    fn run(f: &mut Fx, rule: &str, have: &[&str], age: Option<Age>) -> bool {
        let r: Rule = {
            let mut c: Compiler = Compiler::new(&mut f.items, &f.settings, &f.aliases, &f.groups, false);
            c.rule(rule, &sp()).unwrap()
        };
        let mut cnt: Vec<u32> = vec![0; f.items.len()];
        for h in have {
            let i: ItemId = f.items.find(h).unwrap();
            cnt[i] += 1;
        }
        let c: Ctx = Ctx { cnt: &cnt, age, tod: 0, region: 0, q: None };
        eval(&r.node, &c)
    }

    #[test]
    fn boolean_structure() {
        let mut f: Fx = fx(&[], "{}");
        assert!(run(&mut f, "Sword and Bow", &["Sword", "Bow"], None));
        assert!(!run(&mut f, "Sword and Bow", &["Sword"], None));
        assert!(run(&mut f, "Sword or Bow", &["Bow"], None));
        assert!(run(&mut f, "not Sword", &[], None));
        assert!(run(&mut f, "Sword and (Bow or Key)", &["Sword", "Key"], None));
        assert!(!run(&mut f, "Sword and (Bow or Key)", &["Bow", "Key"], None));
        assert!(run(&mut f, "True", &[], None));
        assert!(!run(&mut f, "False or False", &[], None));
    }

    #[test]
    fn same_op_groups_merge_into_one_check() {
        let mut f: Fx = fx(&[], "{}");
        let mut c: Compiler = Compiler::new(&mut f.items, &f.settings, &f.aliases, &f.groups, false);
        let r: Rule = c.rule("Sword or Bow or Key or Sword", &sp()).unwrap();
        assert!(matches!(&r.node, Node::Any(v) if v.len() == 3));
        let q: Rule = c.rule("Sword and Bow and 'Key'", &sp()).unwrap();
        assert!(matches!(&q.node, Node::All(v) if v.len() == 3));
        assert_eq!(q.deps.len(), 3);
        assert!(!q.tod);
    }

    #[test]
    fn constant_folding_marks_always_and_never() {
        let mut f: Fx = fx(&[("open", Val::Bool(true)), ("mode", Val::Str("hard".to_string()))], "{}");
        let mut c: Compiler = Compiler::new(&mut f.items, &f.settings, &f.aliases, &f.groups, false);
        assert!(c.rule("open or Sword", &sp()).unwrap().always);
        assert!(c.rule("not open and Sword", &sp()).unwrap().never);
        assert!(c.rule("mode == 'hard'", &sp()).unwrap().always);
        assert!(c.rule("mode != 'hard' and Sword", &sp()).unwrap().never);
        assert!(c.rule("mode in ['easy', 'hard']", &sp()).unwrap().always);
        let r: Rule = c.rule("open and Sword", &sp()).unwrap();
        assert!(!r.always && !r.never);
        assert!(matches!(r.node, Node::Has(_, 1)));
    }

    #[test]
    fn counts_and_tuples() {
        let mut f: Fx = fx(&[("need", Val::Int(3))], "{}");
        assert!(!run(&mut f, "(Key, 3)", &["Key", "Key"], None));
        assert!(run(&mut f, "(Key, 3)", &["Key", "Key", "Key"], None));
        assert!(run(&mut f, "(Key, need)", &["Key", "Key", "Key"], None));
        assert!(!run(&mut f, "(Key, need)", &["Key", "Key"], None));
        assert!(run(&mut f, "item_count(Key) >= 2", &["Key", "Key"], None));
        assert!(run(&mut f, "count_distinct((Key, Sword, Bow)) == 2", &["Key", "Key", "Sword"], None));
    }

    #[test]
    fn age_rules() {
        let mut f: Fx = fx(&[], r#"{"is_child": "age == 'child'", "is_adult": "age == 'adult'"}"#);
        assert!(run(&mut f, "is_child", &[], Some(Age::Child)));
        assert!(!run(&mut f, "is_child", &[], Some(Age::Adult)));
        assert!(run(&mut f, "is_adult and Bow", &["Bow"], Some(Age::Adult)));
        assert!(!run(&mut f, "is_adult", &[], None));
        assert!(run(&mut f, "age != 'child'", &[], Some(Age::Adult)));
    }

    #[test]
    fn helpers_with_arguments_use_name_comparison() {
        let h: &str = r#"{
            "Hookshot": "Hookshot_Piece",
            "is_adult": "age == 'adult'",
            "magic(item)": "item == Dins_Fire",
            "can_use(item)": "(magic(item) and item and Ocarina) or (is_adult and item and not magic(item))"
        }"#;
        let mut f: Fx = fx(&[], h);
        assert!(run(&mut f, "can_use(Dins_Fire)", &["Dins Fire", "Ocarina"], Some(Age::Child)));
        assert!(!run(&mut f, "can_use(Dins_Fire)", &["Dins Fire"], Some(Age::Child)));
        assert!(run(&mut f, "can_use(Bow)", &["Bow"], Some(Age::Adult)));
        assert!(!run(&mut f, "can_use(Bow)", &["Bow"], Some(Age::Child)));
        assert!(run(&mut f, "can_use(Hookshot)", &["Hookshot Piece"], Some(Age::Adult)));
    }

    #[test]
    fn helper_errors_are_reported() {
        let mut f: Fx = fx(&[], r#"{"two(a, b)": "a and b"}"#);
        let mut c: Compiler = Compiler::new(&mut f.items, &f.settings, &f.aliases, &f.groups, false);
        assert!(c.rule("two(Sword)", &sp()).is_err());
        assert!(c.rule("two", &sp()).is_err());
        assert!(c.rule("nonsense_name", &sp()).is_err());
        assert!(c.rule("no_such_fn(Sword)", &sp()).is_err());
        assert!(c.rule("Sword and", &sp()).is_err());
    }

    #[test]
    fn runaway_helper_expansion_is_stopped() {
        let mut h: String = String::from("{");
        for i in 0..40 {
            h.push_str(&format!("\"h{i}\": \"h{} and h{}\",", i + 1, i + 1));
        }
        h.push_str("\"h40\": \"Sword\", \"loop\": \"loop\"}");
        let mut f: Fx = fx(&[], &h);
        let mut c: Compiler = Compiler::new(&mut f.items, &f.settings, &f.aliases, &f.groups, false);
        assert!(c.rule("h0", &sp()).is_err());
        assert!(c.rule("loop", &sp()).is_err());
        assert!(c.rule("h30", &sp()).is_ok());
    }

    #[test]
    fn lenient_mode_swallows_unknowns() {
        let mut f: Fx = fx(&[], "{}");
        let mut c: Compiler = Compiler::new(&mut f.items, &f.settings, &f.aliases, &f.groups, true);
        let r: Rule = c.rule("unknown_flag or no_such_fn(Sword)", &sp()).unwrap();
        assert!(r.never);
    }

    #[test]
    fn at_and_here_make_cached_subrules() {
        let mut f: Fx = fx(&[], "{}");
        let mut c: Compiler = Compiler::new(&mut f.items, &f.settings, &f.aliases, &f.groups, false);
        let a: Rule = c.rule("at('Far', Sword) and here(Bow)", &sp()).unwrap();
        let b: Rule = c.rule("at('Far', Sword)", &sp()).unwrap();
        assert_eq!(c.subs.len(), 2);
        assert_eq!(c.subs[0].region, "Far");
        assert_eq!(c.subs[0].name, "Far Subrule 1");
        assert_eq!(c.subs[1].region, "R");
        assert_eq!(a.deps.len(), 2);
        assert_eq!(b.deps.len(), 1);
    }

    #[test]
    fn arithmetic_ternary_and_tables() {
        let mut f: Fx = fx(&[("n", Val::Int(7))], "{}");
        assert!(run(&mut f, "n // 2 == 3 and n % 4 == 3 and -n < 0", &[], None));
        assert!(run(&mut f, "(Sword) if n > 5 else (Bow)", &["Sword"], None));
        assert!(!run(&mut f, "(Sword) if n < 5 else (Bow)", &["Sword"], None));
        let mut tbl: std::collections::BTreeMap<String, Val> = std::collections::BTreeMap::new();
        tbl.insert("Forest Temple".to_string(), Val::Bool(true));
        let mut g: Fx = fx(&[("mq", Val::Map(tbl))], "{}");
        assert!(run(&mut g, "mq[Forest_Temple]", &[], None));
        assert!(run(&mut g, "mq['Forest Temple'] and Sword", &["Sword"], None));
    }

    #[test]
    fn time_of_day_nodes_follow_the_setting() {
        let mut off: Fx = fx(&[], "{}");
        let mut c: Compiler = Compiler::new(&mut off.items, &off.settings, &off.aliases, &off.groups, false);
        assert!(c.rule("at_night", &sp()).unwrap().always);
        let mut on: Fx = fx(&[("ensure_tod_access", Val::Bool(true))], r#"{"skip_to_night": "Ocarina"}"#);
        let mut c2: Compiler = Compiler::new(&mut on.items, &on.settings, &on.aliases, &on.groups, false);
        let r: Rule = c2.rule("at_night and Sword", &sp()).unwrap();
        assert!(r.tod);
        assert!(!r.always);
        let cnt: Vec<u32> = vec![0; on.items.len()];
        let ctx: Ctx = Ctx { cnt: &cnt, age: Some(Age::Adult), tod: DAMPE, region: 0, q: None };
        assert!(!eval(&r.node, &ctx));
        let mut have: Vec<u32> = vec![0; on.items.len()];
        have[on.items.find("Sword").unwrap()] = 1;
        let ctx2: Ctx = Ctx { cnt: &have, age: Some(Age::Adult), tod: DAMPE, region: 0, q: None };
        assert!(eval(&r.node, &ctx2));
        let ctx3: Ctx = Ctx { cnt: &have, age: Some(Age::Adult), tod: DAY, region: 0, q: None };
        assert!(!eval(&r.node, &ctx3));
    }
}
