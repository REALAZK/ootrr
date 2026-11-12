use crate::byml::{Byml, Node};
use crate::error::{Error, Res};
use crate::teleport_dest::{Dest, start_name};

pub const FILE: &str = "Banc/BancBasedResource/StartPos.byml.zs";

fn vec3(v: [f32; 3]) -> Node {
    Node::Arr(v.iter().map(|x| Node::F32(*x)).collect())
}

pub fn add(b: &mut Byml, ds: &[Dest]) -> Res<usize> {
    let mut n: usize = 0;
    for d in ds {
        let name: String = start_name(d);
        let g: &mut Node = b
            .root
            .get_mut(&d.banc)
            .ok_or_else(|| Error::Msg(format!("teleport: StartPos has no group for map {}", d.banc)))?;
        if g.get(&name).is_some() {
            return Err(Error::Msg(format!("teleport: {}/{name} already exists", d.banc)));
        }
        let e: Node = Node::Map(vec![
            ("Rot".to_string(), vec3([0.0, d.yaw, 0.0])),
            ("Trans".to_string(), vec3(d.pos)),
        ]);
        if !g.set(&name, e) {
            return Err(Error::Format("teleport: a StartPos group is not a map"));
        }
        n += 1;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Byml {
        let g: Node = Node::Map(vec![("EntrancePos".to_string(), Node::Map(vec![("Rot".to_string(), vec3([0.0; 3])), ("Trans".to_string(), vec3([1.0, 2.0, 3.0]))]))]);
        Byml { root: Node::Map(vec![("FireTemple".to_string(), g)]), version: 7, big: false }
    }

    fn dest(id: &str, banc: &str) -> Dest {
        Dest { id: id.to_string(), banc: banc.to_string(), pos: [4.0, 5.0, 6.0], yaw: 1.0, label: String::new() }
    }

    #[test]
    fn adds_and_round_trips() {
        let mut b: Byml = sample();
        assert_eq!(add(&mut b, &[dest("A", "FireTemple"), dest("B", "FireTemple")]).unwrap(), 2);
        let w: Vec<u8> = b.write().unwrap();
        assert_eq!(Byml::parse(&w).unwrap(), b);
        let g: &Node = b.root.get("FireTemple").unwrap();
        assert_eq!(g.get("TP_A").unwrap().get("Trans").unwrap(), &vec3([4.0, 5.0, 6.0]));
    }

    #[test]
    fn refuses_unknown_maps_and_clashes() {
        let mut b: Byml = sample();
        assert!(add(&mut b, &[dest("A", "Nowhere")]).is_err());
        assert!(add(&mut b, &[dest("A", "FireTemple"), dest("A", "FireTemple")]).is_err());
    }
}
