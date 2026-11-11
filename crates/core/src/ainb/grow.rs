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
}

#[cfg(test)]
mod tests {
    use super::super::tests::{node, synth};
    use super::super::{audit, Ainb};
    use super::*;

    fn flag(v: bool) -> Vec<u8> {
        u32::from(v).to_le_bytes().to_vec()
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
