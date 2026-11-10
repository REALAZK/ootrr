use crate::error::{Error, Res};

pub struct Rd<'a> {
    b: &'a [u8],
}

impl<'a> Rd<'a> {
    pub fn new(b: &'a [u8]) -> Rd<'a> {
        Rd { b }
    }

    pub fn len(&self) -> usize {
        self.b.len()
    }

    pub fn bytes(&self, o: usize, n: usize) -> Res<&'a [u8]> {
        let e: usize = o.checked_add(n).ok_or(Error::Format("ainb: offset overflow"))?;
        self.b.get(o..e).ok_or(Error::Format("ainb: read past end"))
    }

    pub fn span(&self, a: usize, e: usize) -> Res<&'a [u8]> {
        if e < a {
            return Err(Error::Format("ainb: sections out of order"));
        }
        self.bytes(a, e - a)
    }

    pub fn u16(&self, o: usize) -> Res<u16> {
        let s: &[u8] = self.bytes(o, 2)?;
        Ok(u16::from_le_bytes([s[0], s[1]]))
    }

    pub fn u32(&self, o: usize) -> Res<u32> {
        let s: &[u8] = self.bytes(o, 4)?;
        Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }

    pub fn ptr(&self, o: usize) -> Res<usize> {
        Ok(self.u32(o)? as usize)
    }
}
