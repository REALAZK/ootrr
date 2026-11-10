use ootrr_core::error::{Error, Res};
use ootrr_web::{DEFAULT_PORT, Handle, Opts, opener, start};

struct Args {
    port: u16,
    explicit: bool,
    lan: bool,
    open: bool,
}

fn parse(a: &[String]) -> Res<Args> {
    let mut o: Args = Args { port: DEFAULT_PORT, explicit: false, lan: false, open: true };
    let mut i: usize = 0;
    while i < a.len() {
        match a[i].as_str() {
            "--port" => {
                i += 1;
                let v: &str = a.get(i).ok_or(Error::Format("--port needs a number"))?;
                o.port = v.parse().map_err(|_| Error::Format("--port needs a number from 0 to 65535"))?;
                o.explicit = true;
            }
            "--lan" => o.lan = true,
            "--no-open" => o.open = false,
            other => return Err(Error::Msg(format!("unknown option {other}"))),
        }
        i += 1;
    }
    Ok(o)
}

pub fn run(a: &[String]) -> Res<()> {
    let o: Args = parse(a)?;
    let mut h: Handle = start(&Opts { port: o.port, lan: o.lan, fallback: !o.explicit })?;
    let url: String = h.url();
    println!("OOTRR is running at {url}");
    println!("Keep this window open. Press Ctrl+C to stop.");
    if o.lan {
        println!();
        println!("WARNING: --lan lets other devices on your network reach this server.");
        println!("Traffic is plain HTTP, so the session token can be read by anyone on the network.");
        println!("The page can list folders and start jobs on this computer. Use it only on a network you trust.");
        println!("From another device, open http://<this computer's address>:{}/#token=<the token in the link above>", h.port);
    }
    if o.open && !opener::browser(&url) {
        println!("Could not open a browser. Open the link above by hand.");
    }
    h.wait();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn options_parse() {
        let d: Args = parse(&[]).unwrap();
        assert_eq!(d.port, DEFAULT_PORT);
        assert!(!d.explicit && !d.lan && d.open);
        let o: Args = parse(&v(&["--port", "0", "--lan", "--no-open"])).unwrap();
        assert_eq!(o.port, 0);
        assert!(o.explicit && o.lan && !o.open);
    }

    #[test]
    fn bad_options_are_errors() {
        for a in [v(&["--port"]), v(&["--port", "x"]), v(&["--port", "70000"]), v(&["--wat"])] {
            assert!(parse(&a).is_err());
        }
    }
}
