pub const ASPECT: f32 = 2000.0 / 1440.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cap {
    pub x: f32,
    pub z: f32,
    pub h: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Floors {
    pub min: i32,
    pub ys: Vec<f32>,
}

pub fn uv(c: &Cap, x: f32, z: f32) -> (f32, f32) {
    let u: f32 = 0.5 + (x - c.x) / c.h;
    let v: f32 = 0.5 + (z - c.z) / c.h * ASPECT;
    (u, v)
}

pub fn inside(u: f32, v: f32) -> bool {
    (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v)
}

fn by_height(fl: &Floors, y: f32, allow: &[usize]) -> Option<usize> {
    let mut best: Option<usize> = None;
    for (i, t) in fl.ys.iter().enumerate() {
        if !allow.is_empty() && !allow.contains(&i) {
            continue;
        }
        if *t <= y + 0.5 && best.map(|b| fl.ys[b] <= *t).unwrap_or(true) {
            best = Some(i);
        }
    }
    best
}

pub fn floor_of(fl: &Floors, y: f32, room: &[i32]) -> Option<usize> {
    if fl.ys.is_empty() {
        return None;
    }
    let n: usize = fl.ys.len();
    let allow: Vec<usize> = room
        .iter()
        .filter_map(|f| usize::try_from(f - fl.min).ok())
        .filter(|i| *i < n)
        .collect();
    let plain: usize = by_height(fl, y, &[]).unwrap_or(0);
    if allow.is_empty() || allow.contains(&plain) {
        return Some(plain);
    }
    if allow.len() == 1 {
        return Some(allow[0]);
    }
    Some(by_height(fl, y, &allow).unwrap_or(allow[0]))
}

pub fn label(min: i32, i: usize) -> String {
    let f: i32 = i as i32 + min;
    if f < 0 { format!("B{}", -f) } else { format!("{}F", f + 1) }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KF: Cap = Cap { x: -5.0, z: -12.0, h: 300.0 };

    #[test]
    fn the_camera_centre_is_the_image_centre() {
        let (u, v): (f32, f32) = uv(&KF, -5.0, -12.0);
        assert!((u - 0.5).abs() < 1e-6 && (v - 0.5).abs() < 1e-6);
    }

    #[test]
    fn height_is_the_world_width_of_the_image() {
        let (u, _): (f32, f32) = uv(&KF, -5.0 + 150.0, -12.0);
        assert!((u - 1.0).abs() < 1e-6);
        let (_, v): (f32, f32) = uv(&KF, -5.0, -12.0 + 108.0);
        assert!((v - 1.0).abs() < 1e-5);
        let (a, b): (f32, f32) = uv(&KF, -5.0 - 150.0, -12.0 - 108.0);
        assert!(a.abs() < 1e-6 && b.abs() < 1e-5);
        assert!(inside(0.0, 1.0) && !inside(-0.01, 0.5) && !inside(0.5, 1.01));
    }

    #[test]
    fn floors_follow_height_and_labels_follow_min_floor() {
        let fl: Floors = Floors { min: -2, ys: vec![82.6, 100.0] };
        assert_eq!(floor_of(&fl, 87.0, &[]), Some(0));
        assert_eq!(floor_of(&fl, 99.6, &[]), Some(1));
        assert_eq!(floor_of(&fl, 50.0, &[]), Some(0));
        assert_eq!(label(-2, 0), "B2");
        assert_eq!(label(-2, 1), "B1");
        assert_eq!(label(0, 0), "1F");
        assert_eq!(label(-1, 2), "2F");
        assert_eq!(floor_of(&Floors { min: 0, ys: Vec::new() }, 1.0, &[]), None);
    }

    #[test]
    fn the_room_floor_wins_over_height() {
        let fl: Floors = Floors { min: 0, ys: vec![0.0, 10.0, 20.0] };
        assert_eq!(floor_of(&fl, 25.0, &[0]), Some(0));
        assert_eq!(floor_of(&fl, 25.0, &[0, 1]), Some(1));
        assert_eq!(floor_of(&fl, 12.0, &[1, 2]), Some(1));
        assert_eq!(floor_of(&fl, 12.0, &[9]), Some(1));
    }
}
