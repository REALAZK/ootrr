use crate::error::Error as E;
use crate::slice::fill::SHORT;
use crate::slice::skulls::Mode as Skulls;
use crate::slice::support::{Mode, Plan};
use crate::slice::watch::{OVER, STOPPED};

pub fn changes(p: &Plan) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    match p.skulls {
        Skulls::Matched => out.push("set Gold Skulltulas to Confident matches"),
        Skulls::Pinned => out.push("set Gold Skulltulas to Vanilla"),
        Skulls::Off => {}
    }
    if p.rewards {
        out.push("set Story and NPC rewards to Vanilla");
    }
    if p.drops.any() {
        out.push("shuffle fewer pots and crates");
    }
    if p.world != Mode::Off {
        out.push("shuffle fewer freestanding items");
    }
    out
}

fn options(p: &Plan) -> String {
    let c: Vec<&str> = changes(p);
    match c.len() {
        0 => String::new(),
        1 => c[0].to_string(),
        n => format!("{}, or {}", c[..n - 1].join(", "), c[n - 1]),
    }
}

fn fill_failure(m: &str) -> bool {
    m.starts_with(SHORT) || m.starts_with("no spot for ") || m.starts_with(OVER) || m == "the game is not beatable after the fill"
}

pub fn explain(p: &Plan, e: E) -> E {
    let m: String = match &e {
        E::Format(t) => t.to_string(),
        other => other.to_string(),
    };
    if !fill_failure(&m) || m == STOPPED {
        return e;
    }
    if m.starts_with(SHORT) {
        return E::Msg(format!(
            "No seed can be made with these settings, {m}. Change the settings that decide where these items may go, or shuffle more locations."
        ));
    }
    let o: String = options(p);
    if o.is_empty() {
        return E::Msg(format!("No seed was made with these settings, {m}. Try another seed text."));
    }
    E::Msg(format!("No seed was made with these settings, {m}. Try another seed text, or {o}."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Settings, Value};
    use crate::slice::support::plan;

    fn plan_of(pairs: &[(&str, &str)]) -> Plan {
        let mut s: Settings = Settings::default();
        for (k, v) in pairs {
            s.set(k, Value::s(v)).unwrap();
        }
        plan(&s).unwrap()
    }

    #[test]
    fn the_advice_names_the_settings_that_crowd_the_pool() {
        let p: Plan = plan_of(&[("shuffle_rewards", "supported"), ("shuffle_skulltulas", "matched")]);
        let t: String = explain(&p, E::Msg(format!("{SHORT}: 3 items (Boss Key (Forest Temple)) can only go to 0 free locations"))).to_string();
        assert!(t.starts_with("No seed can be made with these settings, the shuffled items"), "{t}");
        assert!(t.ends_with("or shuffle more locations."), "{t}");
        let l: String = explain(&p, E::Msg(format!("{OVER} of 90 seconds"))).to_string();
        assert!(l.contains("Try another seed text, or set Gold Skulltulas to Confident matches, or set Story and NPC rewards to Vanilla."), "{l}");
        let f: String = explain(&p, E::Format("the game is not beatable after the fill")).to_string();
        assert!(f.starts_with("No seed was made with these settings, the game is not beatable"), "{f}");
        assert!(!l.contains(';') && !t.contains(';') && !t.contains(".."));
    }

    #[test]
    fn plando_and_cancel_messages_pass_through() {
        let p: Plan = plan_of(&[]);
        assert_eq!(explain(&p, E::Msg("plando: x".into())).to_string(), "plando: x");
        assert_eq!(explain(&p, E::Msg(STOPPED.into())).to_string(), STOPPED);
        assert_eq!(explain(&p, E::Msg("no spot for X among 3 free locations".into())).to_string(), "No seed was made with these settings, no spot for X among 3 free locations. Try another seed text.");
    }
}
