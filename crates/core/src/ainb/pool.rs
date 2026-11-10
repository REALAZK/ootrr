#[derive(Clone, Debug, PartialEq, Default)]
pub struct Pool {
    pub b: Vec<u8>,
}

impl Pool {
    pub fn get(&self, o: u32) -> Option<&str> {
        let s: &[u8] = self.b.get(o as usize..)?;
        let e: usize = s.iter().position(|&c| c == 0)?;
        std::str::from_utf8(&s[..e]).ok()
    }

    pub fn find(&self, t: &str) -> Option<u32> {
        let mut s: usize = 0;
        while s < self.b.len() {
            let e: usize = s + self.b[s..].iter().position(|&c| c == 0)?;
            if &self.b[s..e] == t.as_bytes() {
                return Some(s as u32);
            }
            s = e + 1;
        }
        None
    }

    pub fn add(&mut self, t: &str) -> u32 {
        if let Some(o) = self.find(t) {
            return o;
        }
        if self.b.last().is_some_and(|&c| c != 0) {
            self.b.push(0);
        }
        let o: u32 = self.b.len() as u32;
        self.b.extend_from_slice(t.as_bytes());
        self.b.push(0);
        o
    }
}
