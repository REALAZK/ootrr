use super::tab::{INSZ, PSZ};
use super::Ainb;
use crate::error::{Error, Res};

pub const BOOL: usize = 1;
pub const STR: usize = 3;
pub const CONST: u32 = 0x180_0000;

impl Ainb {
    pub fn prop_str(&self, i: usize, name: &str) -> Option<&str> {
        let b: usize = self.params.get(i)?.pe[STR][0] as usize;
        let n: usize = self.params.get(i)?.pe[STR][1] as usize;
        (b..b + n).find_map(|g| {
            if self.prop_name(STR, g)? != name {
                return None;
            }
            let e: &[u8] = self.props[STR].get(g * PSZ[STR]..(g + 1) * PSZ[STR])?;
            Some(self.s(u32::from_le_bytes([e[8], e[9], e[10], e[11]])))
        })
    }

    pub fn add_bool_input(&mut self, i: usize, name: &str, v: bool) -> Res<bool> {
        if i >= self.params.len() {
            return Err(Error::Format("ainb: no such element"));
        }
        if self.find_in(i, BOOL, name).is_some() {
            return Ok(false);
        }
        let key: String = name.to_lowercase();
        let ids: Vec<usize> = self.input_ids(i, BOOL);
        let at: usize = ids
            .iter()
            .find(|&&g| self.input(BOOL, g).is_some_and(|x| self.s(x.name).to_lowercase() > key))
            .copied()
            .unwrap_or(self.params[i].ioe[BOOL][0] as usize + self.params[i].ioe[BOOL][1] as usize);
        let nm: u32 = self.pool.add(name);
        let mut rec: Vec<u8> = Vec::with_capacity(INSZ[BOOL]);
        rec.extend_from_slice(&nm.to_le_bytes());
        rec.extend_from_slice(&(-1i16).to_le_bytes());
        rec.extend_from_slice(&0i16.to_le_bytes());
        rec.extend_from_slice(&CONST.to_le_bytes());
        rec.extend_from_slice(&u32::from(v).to_le_bytes());
        let o: usize = at * INSZ[BOOL];
        self.ins[BOOL].splice(o..o, rec);
        self.params[i].ioe[BOOL][1] += 1;
        for j in i + 1..self.params.len() {
            self.params[j].ioe[BOOL][0] += 1;
        }
        Ok(true)
    }

    /// Turns a bool input of element `i` into the constant `v`. A link that fed the input is cut,
    /// both in the input record and in the element's plug list. Returns whether a link was cut.
    pub fn force_bool(&mut self, i: usize, name: &str, v: bool) -> Res<bool> {
        let g: usize = self.find_in(i, BOOL, name).ok_or(Error::Format("ainb: the element has no such bool input"))?;
        let x = self.input(BOOL, g).ok_or(Error::Format("ainb: input out of range"))?;
        let mut cut: bool = false;
        if x.src != -1 {
            let mut hit: Option<(usize, usize)> = None;
            for k in 0..super::param::GROUPS {
                let found: Option<usize> = self.params[i]
                    .group(k)
                    .iter()
                    .position(|q| q.a as i32 == i32::from(x.src) && self.pool.get(q.b) == Some(name));
                if let Some(j) = found {
                    hit = Some((k, j));
                    break;
                }
            }
            let (k, j): (usize, usize) = hit.ok_or(Error::Format("ainb: the linked input has no matching plug"))?;
            self.params[i].remove_plug(k, j)?;
            cut = true;
        }
        let o: usize = g * INSZ[BOOL];
        let rec: &mut [u8] = self.ins[BOOL].get_mut(o..o + INSZ[BOOL]).ok_or(Error::Format("ainb: input out of range"))?;
        rec[4..6].copy_from_slice(&(-1i16).to_le_bytes());
        rec[6..8].copy_from_slice(&0i16.to_le_bytes());
        rec[8..12].copy_from_slice(&CONST.to_le_bytes());
        rec[12..16].copy_from_slice(&u32::from(v).to_le_bytes());
        Ok(cut)
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{node, synth};
    use super::super::{audit, Ainb, Plug};
    use super::*;

    fn flag(v: bool) -> Vec<u8> {
        u32::from(v).to_le_bytes().to_vec()
    }

    fn name_prop(a: &mut Ainb, i: usize, value: &str) {
        let pn: u32 = a.pool.add("InstanceName");
        let pv: u32 = a.pool.add(value);
        a.params[i].pe[STR] = [(a.props[STR].len() / PSZ[STR]) as u32, 1];
        a.props[STR].extend_from_slice(&pn.to_le_bytes());
        a.props[STR].extend_from_slice(&0u32.to_le_bytes());
        a.props[STR].extend_from_slice(&pv.to_le_bytes());
    }

    fn link(a: &mut Ainb, i: usize, name: &str, src: i16, plug: i16) {
        let g: usize = a.find_in(i, BOOL, name).unwrap();
        let o: usize = g * INSZ[BOOL];
        a.ins[BOOL][o + 4..o + 6].copy_from_slice(&src.to_le_bytes());
        a.ins[BOOL][o + 6..o + 8].copy_from_slice(&plug.to_le_bytes());
        a.ins[BOOL][o + 8..o + 12].copy_from_slice(&0u32.to_le_bytes());
    }

    #[test]
    fn the_sword_patch_points_the_step_input_of_the_draw_gate_at_the_press() {
        use crate::door_open as d;
        let mut a: Ainb = synth();
        let att: usize = node(&mut a, d::ATTENTION, 0, &[]);
        name_prop(&mut a, att, d::SWORD);
        let cn: Vec<u8> = a.pool.add(d::CHALLENGE).to_le_bytes().to_vec();
        let sn: Vec<u8> = a.pool.add(d::STEP).to_le_bytes().to_vec();
        let chk: usize = node(&mut a, d::STEP_CHECK, 0, &[(STR, "ChallengeName", cn), (STR, "ChallengeStepName", sn)]);
        let and: usize = node(&mut a, "OperateAND", 0, &[(BOOL, "BoolMulti", flag(false))]);
        link(&mut a, and, "BoolMulti", -100, 2);
        for (n, p) in [(chk as u16, 2u16), (att as u16, 2u16)] {
            a.multi.extend_from_slice(&n.to_le_bytes());
            a.multi.extend_from_slice(&p.to_le_bytes());
            a.multi.extend_from_slice(&0u32.to_le_bytes());
        }
        let req: usize = node(&mut a, d::REQUEST, 0, &[(BOOL, "EventRequestTrigger", flag(false))]);
        name_prop(&mut a, req, d::DRAW);
        link(&mut a, req, "EventRequestTrigger", and as i16, 1);
        assert_eq!(d::multi_entry(&a, 0), Some((chk as u16, 2)));
        d::patch_sword(&mut a).unwrap();
        assert_eq!(d::multi_entry(&a, 0), Some((att as u16, 2)));
        assert_eq!(d::multi_entry(&a, 1), Some((att as u16, 2)));
        assert!(d::patch_sword(&mut a).is_err());
        let b: Vec<u8> = a.write().unwrap();
        let c: Ainb = Ainb::parse(&b).unwrap();
        assert_eq!(c.write().unwrap(), b);
        assert_eq!(d::multi_entry(&c, 0), Some((att as u16, 2)));
        let mut plain: Ainb = synth();
        assert!(d::patch_sword(&mut plain).is_err());
    }

    #[test]
    fn the_pedestal_patch_keeps_the_adult_sword_alive() {
        use crate::door_open as d;
        let mut a: Ainb = synth();
        let src: usize = node(&mut a, "InvBool.module", 0, &[]);
        let x: usize = node(&mut a, d::ATTENTION, 0, &[(BOOL, "Logic_Create", flag(true)), (BOOL, "Logic_Delete", flag(false))]);
        name_prop(&mut a, x, d::ADULT_SWORD);
        link(&mut a, x, "Logic_Delete", src as i16, 0);
        let b: u32 = a.pool.add("Logic_Delete");
        a.params[x].add_plug(0, 0, Plug { a: src as u32, b, extra: Vec::new() }).unwrap();
        d::patch_pedestal(&mut a).unwrap();
        let i = a.input(BOOL, a.find_in(x, BOOL, "Logic_Delete").unwrap()).unwrap();
        assert_eq!((i.src, i.flags, i.val), (-1, CONST, flag(false)));
        assert!(a.params[x].group(0).is_empty());
        assert!(d::patch_pedestal(&mut a).is_err());
        assert!(audit(&a).is_empty());
    }

    #[test]
    fn the_door_patch_turns_linked_inputs_into_constants_and_drops_their_plugs() {
        let mut a: Ainb = synth();
        let src: usize = node(&mut a, "Source", 0, &[]);
        let x: usize = node(
            &mut a,
            crate::door_open::GATE,
            0,
            &[(BOOL, "isOpen", flag(false)), (BOOL, "IsSkipAnim", flag(false)), (BOOL, "isClose", flag(true))],
        );
        let pn: u32 = a.pool.add("InstanceName");
        let pv: u32 = a.pool.add(crate::door_open::DOOR);
        a.params[x].pe[STR] = [(a.props[STR].len() / PSZ[STR]) as u32, 1];
        a.props[STR].extend_from_slice(&pn.to_le_bytes());
        a.props[STR].extend_from_slice(&0u32.to_le_bytes());
        a.props[STR].extend_from_slice(&pv.to_le_bytes());
        for name in crate::door_open::INPUTS {
            let g: usize = a.find_in(x, BOOL, name).unwrap();
            let o: usize = g * INSZ[BOOL];
            a.ins[BOOL][o + 4..o + 6].copy_from_slice(&(src as i16).to_le_bytes());
            a.ins[BOOL][o + 6..o + 8].copy_from_slice(&1i16.to_le_bytes());
            a.ins[BOOL][o + 8..o + 12].copy_from_slice(&0u32.to_le_bytes());
            let b: u32 = a.pool.add(name);
            let n: usize = a.params[x].group(0).len();
            a.params[x].add_plug(0, n, Plug { a: src as u32, b, extra: Vec::new() }).unwrap();
        }
        assert_eq!(a.params[x].group(0).len(), 2);
        assert_eq!(crate::door_open::door_element(&a), Some(x));
        assert_eq!(crate::door_open::patch(&mut a).unwrap(), 2);
        assert!(a.params[x].group(0).is_empty());
        for name in crate::door_open::INPUTS {
            let i = a.input(BOOL, a.find_in(x, BOOL, name).unwrap()).unwrap();
            assert_eq!(i.src, -1, "{name}");
            assert_eq!(i.flags, CONST, "{name}");
            assert_eq!(i.val, flag(true), "{name}");
        }
        assert_eq!(a.input(BOOL, a.find_in(x, BOOL, "isClose").unwrap()).unwrap().val, flag(true));
        assert!(audit(&a).is_empty());
        let b: Vec<u8> = a.write().unwrap();
        let c: Ainb = Ainb::parse(&b).unwrap();
        assert_eq!(c.write().unwrap(), b);
        assert!(crate::door_open::patch(&mut a).is_err());
        let mut plain: Ainb = synth();
        assert!(crate::door_open::patch(&mut plain).is_err());
        assert!(crate::door_open::door_element(&plain).is_none());
    }

    #[test]
    fn a_bool_input_lands_in_name_order_and_later_elements_shift() {
        let mut a: Ainb = synth();
        let x: usize = node(&mut a, "Probe", 0, &[(1, "Alpha", flag(false)), (1, "Zulu", flag(false))]);
        let y: usize = node(&mut a, "Probe2", 0, &[(1, "Other", flag(true))]);
        assert!(audit(&a).is_empty());
        assert!(a.add_bool_input(x, "Mike", true).unwrap());
        assert!(audit(&a).is_empty());
        let names: Vec<String> = a.input_ids(x, BOOL).into_iter().map(|g| a.s(a.input(BOOL, g).unwrap().name).to_string()).collect();
        assert_eq!(names, vec!["Alpha", "Mike", "Zulu"]);
        let m: usize = a.find_in(x, BOOL, "Mike").unwrap();
        assert_eq!(a.input(BOOL, m).unwrap().val, flag(true));
        assert_eq!(a.input(BOOL, a.find_in(y, BOOL, "Other").unwrap()).unwrap().val, flag(true));
        assert!(!a.add_bool_input(x, "Mike", false).unwrap());
        let b: Vec<u8> = a.write().unwrap();
        let c: Ainb = Ainb::parse(&b).unwrap();
        assert_eq!(c.write().unwrap(), b);
        assert_eq!(c.params, a.params);
        assert_eq!(c.ins, a.ins);
        assert!(audit(&c).is_empty());
    }

    #[test]
    fn a_name_that_sorts_last_goes_to_the_end() {
        let mut a: Ainb = synth();
        let x: usize = node(&mut a, "Probe", 0, &[(1, "Logic_Create", flag(true)), (1, "RespawnTrigger", flag(false))]);
        a.add_bool_input(x, "Unlock", true).unwrap();
        let names: Vec<String> = a.input_ids(x, BOOL).into_iter().map(|g| a.s(a.input(BOOL, g).unwrap().name).to_string()).collect();
        assert_eq!(names, vec!["Logic_Create", "RespawnTrigger", "Unlock"]);
        assert!(audit(&a).is_empty());
    }
}
