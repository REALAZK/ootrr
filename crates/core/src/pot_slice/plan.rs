use crate::settings::Settings;
use crate::slice::support::Mode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Drops {
    pub pots: Mode,
    pub crates: Mode,
}

fn mode_of(s: &Settings, id: &str, bad: &mut Vec<String>) -> Mode {
    match s.text(id) {
        Ok("off") => Mode::Off,
        Ok("overworld") => Mode::Overworld,
        Ok("dungeons") => Mode::Dungeons,
        Ok("all") => Mode::All,
        Ok(v) => {
            bad.push(format!("{id} is {v}, which is not a known choice"));
            Mode::Off
        }
        Err(e) => {
            bad.push(e.to_string());
            Mode::Off
        }
    }
}

pub fn read(s: &Settings, bad: &mut Vec<String>) -> Drops {
    Drops { pots: mode_of(s, "shuffle_pots", bad), crates: mode_of(s, "shuffle_crates", bad) }
}

impl Drops {
    pub fn off() -> Drops {
        Drops { pots: Mode::Off, crates: Mode::Off }
    }

    pub fn any(&self) -> bool {
        self.pots != Mode::Off || self.crates != Mode::Off
    }

    pub fn notes(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        if self.pots != Mode::Off {
            out.push("Pots and flying pots hold shuffled items, only the pots whose spot was matched with confidence and only items the game ships as world pickups. A shuffled pot gives its item once and never refills, its saved row is made permanent. Experimental and untested in the game.".to_string());
        }
        if self.crates != Mode::Off {
            out.push("Crates hold shuffled items, only the few crates matched with confidence. A shuffled crate gives its item once and never refills. Experimental and untested in the game.".to_string());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Value;

    #[test]
    fn the_defaults_are_off_and_every_mode_reads() {
        let mut bad: Vec<String> = Vec::new();
        let s: Settings = Settings::default();
        assert_eq!(read(&s, &mut bad), Drops::off());
        assert!(!read(&s, &mut bad).any());
        assert!(bad.is_empty());
        let mut t: Settings = Settings::default();
        t.set("shuffle_pots", Value::s("dungeons")).unwrap();
        t.set("shuffle_crates", Value::s("overworld")).unwrap();
        let d: Drops = read(&t, &mut bad);
        assert_eq!((d.pots, d.crates), (Mode::Dungeons, Mode::Overworld));
        assert_eq!(d.notes().len(), 2);
        assert!(Drops::off().notes().is_empty());
    }
}
