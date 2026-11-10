use std::fmt::Write;

use super::cont::{Cont, Val};
use super::flow::{Case, Flow, Kind};
use super::tl::Timeline;
use super::{Actor, Bf};

fn val(v: &Val) -> String {
    match v {
        Val::Arg(s) => format!("arg:{s}"),
        Val::Int(x) => x.to_string(),
        Val::Bool(x) => (*x != 0).to_string(),
        Val::Float(x) => x.to_string(),
        Val::Str(s) => format!("{s:?}"),
        Val::Ints(a) => format!("{a:?}"),
        Val::Bools(a) => format!("{a:?}"),
        Val::Floats(a) => format!("{a:?}"),
        Val::Strs(a) => format!("{a:?}"),
        Val::Actor(a, b) => format!("actor({a},{b})"),
        Val::Key(a, b) => format!("key({a},{b})"),
    }
}

pub fn cont(c: &Option<Cont>) -> String {
    match c {
        None => String::new(),
        Some(c) => {
            let v: Vec<String> = c.items.iter().map(|(k, x)| format!("{k}={}", val(x))).collect();
            format!(" {{{}}}", v.join(", "))
        }
    }
}

fn cases(c: &[Case]) -> String {
    let v: Vec<String> = c.iter().map(|x| format!("{:#x}->{}", x.val, x.ev)).collect();
    v.join(" ")
}

fn nm(a: &[Actor], i: u16, k: bool, j: u16) -> String {
    let Some(x) = a.get(i as usize) else { return format!("?{i}.{j}") };
    let l: &Vec<String> = if k { &x.actions } else { &x.queries };
    format!("{}.{}", x.name, l.get(j as usize).map(|s| s.as_str()).unwrap_or("?"))
}

fn flow(o: &mut String, f: &Flow) {
    let _ = writeln!(o, "flow {}", f.name);
    for (i, a) in f.actors.iter().enumerate() {
        let _ = writeln!(
            o,
            " actor {i} {} sub={:?} arg={:?} acts={:?} queries={:?}{}",
            a.name, a.sub, a.arg, a.actions, a.queries, cont(&a.params)
        );
    }
    for e in &f.entries {
        let _ = writeln!(o, " entry {} main={} subs={:?} vars={}", e.name, e.main, e.subs, e.vars.len());
    }
    for (i, e) in f.events.iter().enumerate() {
        let d: String = match &e.kind {
            Kind::Action { next, actor, act, params } => {
                format!("action {} next={next}{}", nm(&f.actors, *actor, true, *act), cont(params))
            }
            Kind::Switch { actor, query, cases: c, params } => {
                format!("switch {} [{}]{}", nm(&f.actors, *actor, false, *query), cases(c), cont(params))
            }
            Kind::Fork { join, forks } => format!("fork join={join} {forks:?}"),
            Kind::Join { next } => format!("join next={next}"),
            Kind::Sub { next, params, flow, entry } => {
                format!("sub {flow}:{entry} next={next}{}", cont(params))
            }
            Kind::SubSwitch { cases: c, params, flow, entry } => {
                format!("subswitch {flow}:{entry} [{}]{}", cases(c), cont(params))
            }
            Kind::Label { hash } => format!("label {hash:#x}"),
        };
        let _ = writeln!(o, " event {i} {} {d}", e.name);
    }
}

fn timeline(o: &mut String, t: &Timeline) {
    let _ = writeln!(o, "timeline {} dur={}", t.name, t.dur);
    for (i, a) in t.actors.iter().enumerate() {
        let _ = writeln!(o, " actor {i} {} acts={:?}{}", a.name, a.actions, cont(&a.params));
    }
    for (i, c) in t.clips.iter().enumerate() {
        let _ = writeln!(
            o,
            " clip {i} {} t={} d={}{}",
            nm(&t.actors, c.actor, true, c.act),
            c.start,
            c.dur,
            cont(&c.params)
        );
    }
    for (i, c) in t.shots.iter().enumerate() {
        let _ = writeln!(o, " shot {i} {} t={}{}", nm(&t.actors, c.actor, true, c.act), c.time, cont(&c.params));
    }
    for (i, c) in t.cuts.iter().enumerate() {
        let _ = writeln!(o, " cut {i} {} t={}{}", c.name, c.start, cont(&c.params));
    }
}

pub fn show(b: &Bf) -> String {
    let mut o: String = String::new();
    let _ = writeln!(o, "file {} version {:?}", b.name, b.ver);
    for f in &b.flows {
        flow(&mut o, f);
    }
    for t in &b.tls {
        timeline(&mut o, t);
    }
    o
}
