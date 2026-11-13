use std::io::{self, Write};
use std::sync::Arc;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use ootrr_core::slice::watch::Live;

pub const QUIET: Duration = Duration::from_secs(5);
pub const EVERY: Duration = Duration::from_secs(1);

pub struct Line {
    tx: Option<mpsc::Sender<()>>,
    join: Option<JoinHandle<bool>>,
}

pub fn text(t: Duration, l: &Live) -> String {
    let how: &str = if l.in_fallback() { ", now trying the careful fill" } else { "" };
    format!("generating, {} s, attempt {}{how}", t.as_secs(), l.tries())
}

pub fn start(l: Arc<Live>) -> Line {
    let (tx, rx): (mpsc::Sender<()>, mpsc::Receiver<()>) = mpsc::channel();
    let t0: Instant = Instant::now();
    let join: io::Result<JoinHandle<bool>> = thread::Builder::new().name("gen-progress".to_string()).spawn(move || {
        let mut shown: bool = false;
        loop {
            let wait: Duration = if shown { EVERY } else { QUIET };
            match rx.recv_timeout(wait) {
                Err(RecvTimeoutError::Timeout) => {
                    shown = true;
                    let mut e: io::Stderr = io::stderr();
                    let _ = write!(e, "\r{}   ", text(t0.elapsed(), &l));
                    let _ = e.flush();
                }
                _ => return shown,
            }
        }
    });
    Line { tx: Some(tx), join: join.ok() }
}

impl Line {
    pub fn stop(mut self) {
        self.tx.take();
        if let Some(j) = self.join.take()
            && j.join().unwrap_or(false)
        {
            eprintln!();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn the_line_names_the_time_and_the_attempt() {
        let l: Live = Live::default();
        l.tries.store(340, Ordering::Relaxed);
        assert_eq!(text(Duration::from_secs(12), &l), "generating, 12 s, attempt 340");
        l.fallback.store(true, Ordering::Relaxed);
        assert!(text(Duration::from_secs(12), &l).ends_with("now trying the careful fill"));
    }

    #[test]
    fn a_fast_run_prints_nothing_and_stops_at_once() {
        let t0: Instant = Instant::now();
        start(Arc::new(Live::default())).stop();
        assert!(t0.elapsed() < QUIET);
    }
}
