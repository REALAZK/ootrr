use super::model::Save;
use super::presets::{Applied, Op, apply};
use super::val::Val;
use crate::error::Res;

pub const BANC: &str = "Sequence_CurrentBanc";
pub const BANC_SHORT: &str = "0x4a6bdb7d";
pub const POS: [&str; 3] = ["PlayerStatus.SavePos", "Player_Pos_For_Map", "0x8773f02d"];
pub const RAD_Y: &str = "PlayerStatus.SavePosRadY";
pub const MAP_ROT: &str = "Player_Rot_For_Map";
pub const CHILD: &str = "Sequence_IsChildhood";

pub fn yaw(rot: [f32; 3]) -> f32 {
    let pi: f32 = std::f32::consts::PI;
    if rot[0].abs() > pi / 2.0 {
        let y: f32 = pi - rot[1];
        if y > pi { y - 2.0 * pi } else { y }
    } else {
        rot[1]
    }
}

pub fn ops(map: &str, trans: [f32; 3], rad_y: f32) -> Vec<Op> {
    let mut out: Vec<Op> = vec![
        (BANC.to_string(), Val::Str(map.to_string())),
        (BANC_SHORT.to_string(), Val::Str(map.to_string())),
    ];
    for p in POS {
        out.push((p.to_string(), Val::Vec3(trans)));
    }
    out.push((RAD_Y.to_string(), Val::Float(rad_y)));
    out.push((MAP_ROT.to_string(), Val::Float(rad_y.to_degrees() - 180.0)));
    if map.ends_with("_Adult") {
        out.push((CHILD.to_string(), Val::Bool(false)));
    } else if map.ends_with("_Child") {
        out.push((CHILD.to_string(), Val::Bool(true)));
    }
    out
}

pub fn place(s: &mut Save, map: &str, trans: [f32; 3], rad_y: f32) -> Res<Applied> {
    apply(s, &ops(map, trans, rad_y))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flipped_euler_angles_give_the_real_yaw() {
        let pi: f32 = std::f32::consts::PI;
        assert!((yaw([0.0, 0.5, 0.0]) - 0.5).abs() < 1e-6);
        assert!((yaw([pi, 1.0, pi]) - (pi - 1.0)).abs() < 1e-6);
        assert!((yaw([pi, -1.0, pi]) - (1.0 - pi)).abs() < 1e-5);
    }

    #[test]
    fn age_follows_the_map_suffix() {
        let a: Vec<Op> = ops("HyruleField_Adult", [1.0, 2.0, 3.0], 0.5);
        assert!(a.contains(&(CHILD.to_string(), Val::Bool(false))));
        let c: Vec<Op> = ops("HyruleField_Child", [1.0, 2.0, 3.0], 0.5);
        assert!(c.contains(&(CHILD.to_string(), Val::Bool(true))));
        let d: Vec<Op> = ops("DodongoCavern", [1.0, 2.0, 3.0], 0.5);
        assert!(!d.iter().any(|o| o.0 == CHILD));
    }
}
