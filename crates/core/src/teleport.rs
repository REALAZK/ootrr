use std::f32::consts::PI;
use std::fmt::Write as W;
use std::path::Path as P;

use crate::TITLE_ID;
use crate::bfevfl::Bf;
use crate::byml::Byml;
use crate::error::{Error, Res};
use crate::modpack::Pack;
use crate::romfs::Romfs;
use crate::rsizetable::{self, Rst};
use crate::staging::{self, Out};
use crate::teleport_dest::Dest;
use crate::teleport_flow;
use crate::teleport_start;

pub const FLOW_FILE: &str = "Event/EventFlow/Dm_SY_Ocarina.bfevfl.zs";
pub const FLOW: &str = "Dm_SY_Ocarina";
pub const INDEX_FLAG: &str = "DungeonRoomStep.TestField.DungeonRoom000.Step";
pub const MODE_FLAG: &str = "DungeonRoomStep.TestField.DungeonRoom001.Step";
pub const PREFIX: &str = "OOTRRTp";
pub const MARGIN: u32 = 0x2000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Start,
    Coords,
}

impl Mode {
    pub fn parse(s: &str) -> Res<Mode> {
        match s {
            "start" => Ok(Mode::Start),
            "coords" => Ok(Mode::Coords),
            _ => Err(Error::Format("teleport: mode must be start or coords")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Deg,
    Rad,
}

impl Unit {
    pub fn parse(s: &str) -> Res<Unit> {
        match s {
            "deg" => Ok(Unit::Deg),
            "rad" => Ok(Unit::Rad),
            _ => Err(Error::Format("teleport: rotation unit must be deg or rad")),
        }
    }

    pub fn conv(self, r: f32) -> f32 {
        let full: f32 = match self {
            Unit::Deg => 360.0,
            Unit::Rad => 2.0 * PI,
        };
        let v: f32 = match self {
            Unit::Deg => r.to_degrees(),
            Unit::Rad => r,
        };
        v - full * (v / full).round()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    Direct,
    Shared,
}

impl Layout {
    pub fn parse(s: &str) -> Res<Layout> {
        match s {
            "direct" => Ok(Layout::Direct),
            "shared" => Ok(Layout::Shared),
            _ => Err(Error::Format("teleport: layout must be direct or shared")),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cfg {
    pub mode: Mode,
    pub unit: Unit,
    pub layout: Layout,
    pub groups: bool,
    pub autosave: bool,
    pub flag: String,
    pub mflag: String,
}

impl Cfg {
    pub fn new() -> Cfg {
        Cfg {
            mode: Mode::Start,
            unit: Unit::Deg,
            layout: Layout::Shared,
            groups: true,
            autosave: false,
            flag: INDEX_FLAG.to_string(),
            mflag: MODE_FLAG.to_string(),
        }
    }

    pub fn check(&self) -> Res<()> {
        if self.mode == Mode::Coords && self.layout == Layout::Shared {
            return Err(Error::Format("teleport: the coords mode needs the direct layout"));
        }
        if self.flag == self.mflag {
            return Err(Error::Format("teleport: the index flag and the mode flag must differ"));
        }
        Ok(())
    }
}

impl Default for Cfg {
    fn default() -> Cfg {
        Cfg::new()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub added: usize,
    pub leaves: usize,
    pub groups: usize,
    pub depth: usize,
    pub root: u16,
}

pub fn patch(b: &mut Bf, cfg: &Cfg, ds: &[Dest]) -> Res<Report> {
    cfg.check()?;
    teleport_flow::patch(b, cfg, ds)
}

pub fn checked_bf(raw: &[u8], cfg: &Cfg, ds: &[Dest]) -> Res<Vec<u8>> {
    let mut b: Bf = Bf::parse(raw)?;
    patch(&mut b, cfg, ds)?;
    let out: Vec<u8> = b.write()?;
    if Bf::parse(&out)? != b {
        return Err(Error::Format("teleport: the patched flow does not parse back to the edited model"));
    }
    Ok(out)
}

pub fn checked_byml(raw: &[u8], ds: &[Dest]) -> Res<Vec<u8>> {
    let mut b: Byml = Byml::parse(raw)?;
    teleport_start::add(&mut b, ds)?;
    let out: Vec<u8> = b.write()?;
    if Byml::parse(&out)? != b {
        return Err(Error::Format("teleport: the new StartPos file does not parse back to the edited model"));
    }
    Ok(out)
}

pub fn stage(rf: &mut Romfs, cfg: &Cfg, ds: &[Dest]) -> Res<Vec<Out>> {
    let mut outs: Vec<Out> = Vec::new();
    if cfg.mode == Mode::Start {
        outs.push(staging::rewrite(rf, teleport_start::FILE, |raw| checked_byml(raw, ds))?);
    }
    outs.push(staging::rewrite(rf, FLOW_FILE, |raw| checked_bf(raw, cfg, ds))?);
    Ok(outs)
}

pub fn build(romfs: &P, load: &P, name: &str, cfg: &Cfg, ds: &[Dest]) -> Res<String> {
    let mut rf: Romfs = Romfs::open(romfs)?;
    let outs: Vec<Out> = stage(&mut rf, cfg, ds)?;
    let mut rst: Rst = rsizetable::load(&mut rf)?;
    let pk: Pack = Pack::create(load, TITLE_ID, name)?;
    let mut note: String = String::new();
    for x in &outs {
        let was: Option<u32> = rst.get(&x.path);
        let now: Option<u32> = rst.grow(&x.path, x.old as u64, x.new as u64, MARGIN)?;
        pk.write_romfs(&x.path, &x.z)?;
        let _ = writeln!(note, "{}: {} -> {} bytes, size table {:?} -> {:?}", x.path, x.old, x.new, was, now);
    }
    pk.write_romfs(rsizetable::TABLE_PATH, &rst.to_zs()?)?;
    Ok(note)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_units_convert_and_wrap() {
        assert!((Unit::Deg.conv(PI) - 180.0).abs() < 1e-3 || (Unit::Deg.conv(PI) + 180.0).abs() < 1e-3);
        assert!((Unit::Deg.conv(3.0 * PI / 2.0) + 90.0).abs() < 1e-3);
        assert!((Unit::Rad.conv(3.0 * PI / 2.0) + PI / 2.0).abs() < 1e-4);
        assert!((Unit::Deg.conv(0.5) - 28.6479).abs() < 1e-3);
        assert!(Unit::parse("deg").is_ok() && Unit::parse("rad").is_ok() && Unit::parse("turns").is_err());
    }

    #[test]
    fn mode_and_layout_names_parse() {
        assert_eq!(Mode::parse("start").unwrap(), Mode::Start);
        assert_eq!(Mode::parse("coords").unwrap(), Mode::Coords);
        assert!(Mode::parse("x").is_err());
        assert_eq!(Layout::parse("shared").unwrap(), Layout::Shared);
        assert_eq!(Layout::parse("direct").unwrap(), Layout::Direct);
        assert!(Layout::parse("x").is_err());
    }

    #[test]
    fn config_rules() {
        let mut c: Cfg = Cfg::new();
        assert!(c.check().is_ok());
        c.mode = Mode::Coords;
        assert!(c.check().is_err());
        c.layout = Layout::Direct;
        assert!(c.check().is_ok());
        c.mflag = c.flag.clone();
        assert!(c.check().is_err());
    }
}
