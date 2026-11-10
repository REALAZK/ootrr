use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use ootrr_core::error::{Error, Res};

use crate::error::Fail;
use crate::guard::Guard;
use crate::jobs::Jobs;
use crate::req::{self, Limits};
use crate::resp::{self, Resp};
use crate::router;
use crate::state::Shared;
use crate::token;

pub const DEFAULT_PORT: u16 = 7878;
pub const MAX_CONNECTIONS: usize = 64;
pub const IO_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy)]
pub struct Opts {
    pub port: u16,
    pub lan: bool,
    pub fallback: bool,
}

pub struct Handle {
    pub port: u16,
    pub token: String,
    pub lan: bool,
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl Handle {
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}/#token={}", self.port, self.token)
    }

    pub fn wait(&mut self) {
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect_timeout(&SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), self.port), Duration::from_secs(2));
        self.wait();
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        if self.join.is_some() {
            self.stop();
        }
    }
}

fn bind(o: &Opts) -> Res<TcpListener> {
    let ip: Ipv4Addr = if o.lan { Ipv4Addr::UNSPECIFIED } else { Ipv4Addr::LOCALHOST };
    match TcpListener::bind(SocketAddr::new(IpAddr::V4(ip), o.port)) {
        Ok(l) => Ok(l),
        Err(_) if o.fallback && o.port != 0 => Ok(TcpListener::bind(SocketAddr::new(IpAddr::V4(ip), 0))?),
        Err(e) => Err(Error::Msg(format!("cannot listen on port {}: {e}", o.port))),
    }
}

fn send(s: &mut TcpStream, r: &Resp) {
    let _ = s.write_all(&resp::to_bytes(r));
    let _ = s.flush();
    let _ = s.shutdown(Shutdown::Write);
}

fn serve_one(mut s: TcpStream, sh: &Shared) {
    let _ = s.set_read_timeout(Some(IO_TIMEOUT));
    let _ = s.set_write_timeout(Some(IO_TIMEOUT));
    let r: Resp = match req::read_req(&mut s, &Limits::DEFAULT) {
        Ok(q) => router::handle(sh, &q),
        Err(f) => Resp::err(&f),
    };
    send(&mut s, &r);
}

struct Slot(Arc<AtomicUsize>);

impl Drop for Slot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

pub fn start(o: &Opts) -> Res<Handle> {
    let l: TcpListener = bind(o)?;
    let port: u16 = l.local_addr()?.port();
    let tok: String = token::token();
    let sh: Arc<Shared> = Arc::new(Shared { guard: Guard { port, token: tok.clone(), lan: o.lan }, jobs: Jobs::new() });
    let stop: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let flag: Arc<AtomicBool> = Arc::clone(&stop);
    let active: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let join: JoinHandle<()> = thread::Builder::new().name("ootrr-accept".to_string()).spawn(move || {
        for c in l.incoming() {
            if flag.load(Ordering::SeqCst) {
                break;
            }
            let mut s: TcpStream = match c {
                Ok(s) => s,
                Err(_) => continue,
            };
            if active.fetch_add(1, Ordering::SeqCst) >= MAX_CONNECTIONS {
                active.fetch_sub(1, Ordering::SeqCst);
                send(&mut s, &Resp::err(&Fail::new(503, "too many connections")));
                continue;
            }
            let slot: Slot = Slot(Arc::clone(&active));
            let sh2: Arc<Shared> = Arc::clone(&sh);
            let spawned: std::io::Result<JoinHandle<()>> = thread::Builder::new().name("ootrr-conn".to_string()).spawn(move || {
                let _slot: Slot = slot;
                serve_one(s, &sh2);
            });
            drop(spawned);
        }
    })?;
    Ok(Handle { port, token: tok, lan: o.lan, stop, join: Some(join) })
}
