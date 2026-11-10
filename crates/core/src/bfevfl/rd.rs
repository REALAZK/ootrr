use crate::error::{Error, Res};

pub struct Rd<'a> {
    pub b: &'a [u8],
}

fn cut() -> Error {
    Error::Format("bfevfl: truncated or out of range")
}

impl<'a> Rd<'a> {
    pub fn new(b: &'a [u8]) -> Rd<'a> {
        Rd { b }
    }

    pub fn bytes(&self, o: usize, n: usize) -> Res<&'a [u8]> {
        let e: usize = o.checked_add(n).ok_or_else(cut)?;
        self.b.get(o..e).ok_or_else(cut)
    }

    pub fn u8(&self, o: usize) -> Res<u8> {
        Ok(self.bytes(o, 1)?[0])
    }

    pub fn u16(&self, o: usize) -> Res<u16> {
        let s: &[u8] = self.bytes(o, 2)?;
        Ok(u16::from_le_bytes([s[0], s[1]]))
    }

    pub fn u32(&self, o: usize) -> Res<u32> {
        let s: &[u8] = self.bytes(o, 4)?;
        Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }

    pub fn u64(&self, o: usize) -> Res<u64> {
        let s: &[u8] = self.bytes(o, 8)?;
        let mut a: [u8; 8] = [0; 8];
        a.copy_from_slice(s);
        Ok(u64::from_le_bytes(a))
    }

    pub fn f32(&self, o: usize) -> Res<f32> {
        Ok(f32::from_bits(self.u32(o)?))
    }

    pub fn ptr(&self, o: usize) -> Res<usize> {
        usize::try_from(self.u64(o)?).map_err(|_| cut())
    }

    pub fn pstr(&self, o: usize) -> Res<String> {
        let n: usize = self.u16(o)? as usize;
        let d: &[u8] = self.bytes(o + 2, n)?;
        String::from_utf8(d.to_vec()).map_err(|_| Error::Format("bfevfl: string is not utf-8"))
    }

    pub fn sref(&self, o: usize) -> Res<String> {
        let p: usize = self.ptr(o)?;
        if p == 0 {
            return Err(Error::Format("bfevfl: null string pointer"));
        }
        self.pstr(p)
    }
}
