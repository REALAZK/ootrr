use super::rd::Rd;
use crate::error::{Error, Res};

pub const HEAD: usize = 0xa4;
pub const GROUPS: usize = 10;

#[derive(Clone, Debug, PartialEq)]
pub struct Plug {
    pub a: u32,
    pub b: u32,
    pub extra: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    pub pe: [[u32; 2]; 6],
    pub ioe: [[u32; 4]; 6],
    pub pl: [[u8; 2]; GROUPS],
    pub plugs: Vec<Plug>,
}

impl Param {
    pub fn parse(r: &Rd, s: usize, e: usize) -> Res<Param> {
        if e < s + HEAD {
            return Err(Error::Format("ainb: element parameters too short"));
        }
        let mut pe: [[u32; 2]; 6] = [[0; 2]; 6];
        for k in 0..6 {
            pe[k] = [r.u32(s + 8 * k)?, r.u32(s + 8 * k + 4)?];
        }
        let mut ioe: [[u32; 4]; 6] = [[0; 4]; 6];
        for k in 0..6 {
            for j in 0..4 {
                ioe[k][j] = r.u32(s + 0x30 + 16 * k + 4 * j)?;
            }
        }
        let mut pl: [[u8; 2]; GROUPS] = [[0; 2]; GROUPS];
        let pb: &[u8] = r.bytes(s + 0x90, 2 * GROUPS)?;
        for k in 0..GROUPS {
            pl[k] = [pb[2 * k], pb[2 * k + 1]];
        }
        let n: usize = pl.iter().map(|g| g[0] as usize).sum();
        let mut ptrs: Vec<usize> = Vec::new();
        for j in 0..n {
            ptrs.push(r.ptr(s + HEAD + 4 * j)?);
        }
        let body: usize = s + HEAD + 4 * n;
        if n == 0 && e != body {
            return Err(Error::Format("ainb: stray bytes after element parameters"));
        }
        let mut plugs: Vec<Plug> = Vec::new();
        for j in 0..n {
            let at: usize = ptrs[j];
            let nx: usize = if j + 1 < n { ptrs[j + 1] } else { e };
            if j == 0 && at != body {
                return Err(Error::Format("ainb: plug data not contiguous"));
            }
            if nx < at + 8 {
                return Err(Error::Format("ainb: plug data too short"));
            }
            plugs.push(Plug { a: r.u32(at)?, b: r.u32(at + 4)?, extra: r.bytes(at + 8, nx - at - 8)?.to_vec() });
        }
        Ok(Param { pe, ioe, pl, plugs })
    }

    pub fn size(&self) -> usize {
        HEAD + 4 * self.plugs.len() + self.plugs.iter().map(|p| 8 + p.extra.len()).sum::<usize>()
    }

    pub fn write(&self, out: &mut Vec<u8>) {
        let s: usize = out.len();
        for k in 0..6 {
            out.extend_from_slice(&self.pe[k][0].to_le_bytes());
            out.extend_from_slice(&self.pe[k][1].to_le_bytes());
        }
        for k in 0..6 {
            for j in 0..4 {
                out.extend_from_slice(&self.ioe[k][j].to_le_bytes());
            }
        }
        for g in &self.pl {
            out.extend_from_slice(g);
        }
        let mut at: usize = s + HEAD + 4 * self.plugs.len();
        for p in &self.plugs {
            out.extend_from_slice(&(at as u32).to_le_bytes());
            at += 8 + p.extra.len();
        }
        for p in &self.plugs {
            out.extend_from_slice(&p.a.to_le_bytes());
            out.extend_from_slice(&p.b.to_le_bytes());
            out.extend_from_slice(&p.extra);
        }
    }

    pub fn group(&self, k: usize) -> &[Plug] {
        let b: usize = self.pl[k][1] as usize;
        let n: usize = self.pl[k][0] as usize;
        self.plugs.get(b..b + n).unwrap_or(&[])
    }

    pub fn add_plug(&mut self, k: usize, pos: usize, p: Plug) -> Res<()> {
        let n: usize = self.pl[k][0] as usize;
        let b: usize = self.pl[k][1] as usize;
        if pos > n || n >= 255 || self.plugs.len() >= 255 {
            return Err(Error::Format("ainb: cannot add plug there"));
        }
        self.plugs.insert(b + pos, p);
        self.pl[k][0] += 1;
        for j in k + 1..GROUPS {
            self.pl[j][1] += 1;
        }
        Ok(())
    }

    pub fn remove_plug(&mut self, k: usize, pos: usize) -> Res<Plug> {
        let n: usize = self.pl[k][0] as usize;
        let b: usize = self.pl[k][1] as usize;
        if pos >= n || b + pos >= self.plugs.len() {
            return Err(Error::Format("ainb: no plug there to remove"));
        }
        let p: Plug = self.plugs.remove(b + pos);
        self.pl[k][0] -= 1;
        for j in k + 1..GROUPS {
            self.pl[j][1] -= 1;
        }
        Ok(p)
    }
}
