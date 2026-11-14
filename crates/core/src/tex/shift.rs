const ROOT3: f32 = 1.732_050_8;
const FLOOR: f32 = 10.0;
const FULL: f32 = 40.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shift {
    pub m: [[f32; 3]; 3],
    pub mid: f32,
    pub span: f32,
}

fn curve(v: f32) -> f32 {
    if v <= 0.003_130_8 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 }
}

fn uncurve(v: f32) -> f32 {
    if v <= 0.040_45 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}

pub fn hue(c: [f32; 3]) -> (f32, f32) {
    let a: f32 = c[0] - (c[1] + c[2]) / 2.0;
    let b: f32 = ROOT3 / 2.0 * (c[1] - c[2]);
    (b.atan2(a).to_degrees().rem_euclid(360.0), a.hypot(b))
}

impl Shift {
    pub fn new(turn: f32, sat: f32, gain: f32) -> Shift {
        let (s, c): (f32, f32) = turn.to_radians().sin_cos();
        let k: [[f32; 3]; 3] = [[0.0, -1.0, 1.0], [1.0, 0.0, -1.0], [-1.0, 1.0, 0.0]];
        let mut m: [[f32; 3]; 3] = [[0.0; 3]; 3];
        for (i, row) in m.iter_mut().enumerate() {
            for (j, v) in row.iter_mut().enumerate() {
                let eye: f32 = if i == j { 1.0 } else { 0.0 };
                *v = gain * (1.0 / 3.0 + sat * (c * (eye - 1.0 / 3.0) + s * k[i][j] / ROOT3));
            }
        }
        Shift { m, mid: 0.0, span: 360.0 }
    }

    pub fn same() -> Shift {
        Shift::new(0.0, 1.0, 1.0)
    }

    pub fn band(mut self, mid: f32, span: f32) -> Shift {
        self.mid = mid;
        self.span = span;
        self
    }

    fn weight(&self, c: [f32; 3]) -> f32 {
        if self.span >= 180.0 {
            return 1.0;
        }
        let (h, chroma): (f32, f32) = hue(c);
        let far: f32 = ((h - self.mid + 540.0).rem_euclid(360.0) - 180.0).abs();
        let by_hue: f32 = ((self.span * 1.5 - far) / (self.span * 0.5)).clamp(0.0, 1.0);
        let by_chroma: f32 = ((chroma - FLOOR) / (FULL - FLOOR)).clamp(0.0, 1.0);
        by_hue * by_chroma
    }

    fn turn(&self, c: [f32; 3]) -> [f32; 3] {
        let w: f32 = self.weight(c);
        let mut out: [f32; 3] = [0.0; 3];
        for (i, o) in out.iter_mut().enumerate() {
            let v: f32 = self.m[i][0] * c[0] + self.m[i][1] * c[1] + self.m[i][2] * c[2];
            *o = c[i] + w * (v - c[i]);
        }
        out
    }

    pub fn rgb(&self, c: [u8; 3]) -> [u8; 3] {
        let v: [f32; 3] = self.turn([c[0] as f32, c[1] as f32, c[2] as f32]);
        [v[0].round().clamp(0.0, 255.0) as u8, v[1].round().clamp(0.0, 255.0) as u8, v[2].round().clamp(0.0, 255.0) as u8]
    }

    pub fn light(&self, c: [f32; 3]) -> [f32; 3] {
        let top: f32 = c[0].max(c[1]).max(c[2]).max(1.0);
        let v: [f32; 3] = self.turn([curve(c[0].max(0.0) / top) * 255.0, curve(c[1].max(0.0) / top) * 255.0, curve(c[2].max(0.0) / top) * 255.0]);
        [uncurve((v[0] / 255.0).clamp(0.0, 1.0)) * top, uncurve((v[1] / 255.0).clamp(0.0, 1.0)) * top, uncurve((v[2] / 255.0).clamp(0.0, 1.0)) * top]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_turn_changes_nothing() {
        let s: Shift = Shift::same();
        for c in [[0, 0, 0], [255, 255, 255], [64, 82, 43], [200, 10, 99]] {
            assert_eq!(s.rgb(c), c);
        }
        let l: [f32; 3] = s.light([0.02, 0.3, 2.5]);
        assert!((l[0] - 0.02).abs() < 1e-3 && (l[1] - 0.3).abs() < 1e-3 && (l[2] - 2.5).abs() < 1e-2, "{l:?}");
    }

    #[test]
    fn a_third_of_a_turn_cycles_the_channels_and_keeps_grey() {
        let s: Shift = Shift::new(120.0, 1.0, 1.0);
        assert_eq!(s.rgb([200, 40, 10]), [10, 200, 40]);
        assert_eq!(s.rgb([90, 90, 90]), [90, 90, 90]);
        let back: Shift = Shift::new(-120.0, 1.0, 1.0);
        assert_eq!(back.rgb([10, 200, 40]), [200, 40, 10]);
    }

    #[test]
    fn a_turn_keeps_the_channel_sum() {
        for t in [17.0f32, 95.0, 181.0, 300.0] {
            let s: Shift = Shift::new(t, 1.0, 1.0);
            let v: [f32; 3] = s.turn([64.0, 82.0, 43.0]);
            assert!((v[0] + v[1] + v[2] - 189.0).abs() < 0.01, "{t}");
        }
    }

    #[test]
    fn a_band_only_moves_colours_near_its_hue() {
        let green: (f32, f32) = hue([40.0, 160.0, 50.0]);
        let s: Shift = Shift::new(120.0, 1.0, 1.0).band(green.0, 30.0);
        assert_eq!(s.rgb([40, 160, 50]), [50, 40, 160]);
        assert_eq!(s.rgb([170, 60, 40]), [170, 60, 40]);
        assert_eq!(s.rgb([120, 124, 121]), [120, 124, 121]);
    }

    #[test]
    fn grey_scaling_and_gain_apply() {
        let s: Shift = Shift::new(0.0, 0.0, 0.5);
        assert_eq!(s.rgb([30, 60, 90]), [30, 30, 30]);
    }
}
