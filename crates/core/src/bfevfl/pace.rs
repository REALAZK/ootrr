use super::cont::Val;
use super::flow::Kind;
use super::Bf;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Paced {
    pub waits: usize,
    pub before: i64,
    pub after: i64,
}

pub fn scale_waits(b: &mut Bf, factor: f32, min: i32) -> Paced {
    let mut out: Paced = Paced::default();
    for f in b.flows.iter_mut() {
        for e in f.events.iter_mut() {
            let Kind::Action { params: Some(c), .. } = &mut e.kind else {
                continue;
            };
            for (k, v) in c.items.iter_mut() {
                if k != "Frames" {
                    continue;
                }
                let Val::Int(x) = v else {
                    continue;
                };
                if *x < min {
                    continue;
                }
                let y: i32 = ((*x as f32) * factor).round().max(1.0) as i32;
                out.before += *x as i64;
                out.after += y as i64;
                *x = y;
                out.waits += 1;
            }
        }
    }
    out
}
