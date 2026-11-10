use ootrr_core::error::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct Fail {
    pub status: u16,
    pub msg: String,
}

pub type Hr<T> = Result<T, Fail>;

impl Fail {
    pub fn new(status: u16, msg: &str) -> Fail {
        Fail { status, msg: msg.to_string() }
    }

    pub fn bad(msg: &str) -> Fail {
        Fail::new(400, msg)
    }

    pub fn deny(msg: &str) -> Fail {
        Fail::new(403, msg)
    }
}

impl From<Error> for Fail {
    fn from(e: Error) -> Fail {
        Fail::new(422, &e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_errors_become_422() {
        let f: Fail = Fail::from(Error::Msg("nope".into()));
        assert_eq!(f.status, 422);
        assert_eq!(f.msg, "nope");
        assert_eq!(Fail::bad("x").status, 400);
        assert_eq!(Fail::deny("x").status, 403);
    }
}
