use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;

use ootrr_core::json::Json;

use crate::error::{Fail, Hr};

pub const MAX_KEPT: usize = 32;
pub const MAX_RUNNING: usize = 4;
pub const INLINE: bool = cfg!(target_os = "wasi");

#[derive(Debug, Clone)]
pub struct Step {
    pub label: String,
    pub state: &'static str,
}

#[derive(Debug, Clone)]
pub struct Job {
    pub id: u64,
    pub state: &'static str,
    pub percent: u8,
    pub steps: Vec<Step>,
    pub message: String,
}

pub struct Jobs {
    list: Mutex<Vec<Job>>,
    next: AtomicU64,
}

pub struct Ctl {
    jobs: Arc<Jobs>,
    id: u64,
}

fn finished(s: &str) -> bool {
    s != "running"
}

impl Job {
    pub fn to_json(&self) -> Json {
        let steps: Vec<Json> = self
            .steps
            .iter()
            .map(|s| Json::obj().with("label", s.label.as_str().into()).with("state", s.state.into()))
            .collect();
        Json::obj()
            .with("id", Json::Int(self.id as i64))
            .with("state", self.state.into())
            .with("finished", finished(self.state).into())
            .with("percent", Json::Int(self.percent as i64))
            .with("steps", Json::Arr(steps))
            .with("message", self.message.as_str().into())
    }
}

impl Jobs {
    pub fn new() -> Arc<Jobs> {
        Arc::new(Jobs { list: Mutex::new(Vec::new()), next: AtomicU64::new(1) })
    }

    fn lock(&self) -> MutexGuard<'_, Vec<Job>> {
        self.list.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn edit<F: FnOnce(&mut Job)>(&self, id: u64, f: F) {
        let mut l: MutexGuard<'_, Vec<Job>> = self.lock();
        if let Some(j) = l.iter_mut().find(|j| j.id == id) {
            f(j);
        }
    }

    pub fn get(&self, id: u64) -> Option<Json> {
        self.lock().iter().find(|j| j.id == id).map(Job::to_json)
    }

    #[cfg(test)]
    pub fn running(&self) -> usize {
        self.lock().iter().filter(|j| !finished(j.state)).count()
    }

    pub fn start<F>(self: &Arc<Jobs>, work: F) -> Hr<u64>
    where
        F: FnOnce(&Ctl) + Send + 'static,
    {
        let id: u64 = {
            let mut l: MutexGuard<'_, Vec<Job>> = self.lock();
            if l.iter().filter(|j| !finished(j.state)).count() >= MAX_RUNNING {
                return Err(Fail::new(429, "too many jobs are running, wait for one to finish"));
            }
            let id: u64 = self.next.fetch_add(1, Ordering::Relaxed);
            l.push(Job { id, state: "running", percent: 0, steps: Vec::new(), message: String::new() });
            while l.len() > MAX_KEPT {
                match l.iter().position(|j| finished(j.state)) {
                    Some(i) => {
                        l.remove(i);
                    }
                    None => break,
                }
            }
            id
        };
        let c: Ctl = Ctl { jobs: Arc::clone(self), id };
        let run = move || {
            work(&c);
            c.jobs.edit(id, |j| {
                if !finished(j.state) {
                    j.state = "failed";
                    j.message = "the job ended without a result".to_string();
                }
            });
        };
        if INLINE {
            run();
            return Ok(id);
        }
        let r: std::io::Result<thread::JoinHandle<()>> = thread::Builder::new().name(format!("job-{id}")).spawn(run);
        if r.is_err() {
            self.edit(id, |j| {
                j.state = "failed";
                j.message = "could not start a worker thread".to_string();
            });
            return Err(Fail::new(500, "could not start a worker thread"));
        }
        Ok(id)
    }
}

impl Ctl {
    pub fn begin(&self, label: &str) {
        self.jobs.edit(self.id, |j| j.steps.push(Step { label: label.to_string(), state: "running" }));
    }

    pub fn ok(&self, percent: u8) {
        self.jobs.edit(self.id, |j| {
            if let Some(s) = j.steps.last_mut() {
                s.state = "done";
            }
            j.percent = percent.min(100);
        });
    }

    pub fn mark(&self, state: &'static str) {
        self.jobs.edit(self.id, |j| {
            if let Some(s) = j.steps.last_mut() {
                s.state = state;
            }
        });
    }

    pub fn finish(&self, state: &'static str, message: &str) {
        self.jobs.edit(self.id, |j| {
            j.state = state;
            j.message = message.to_string();
            if state == "done" {
                j.percent = 100;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    fn wait(j: &Arc<Jobs>, id: u64) -> Json {
        for _ in 0..500 {
            if let Some(x) = j.get(id)
                && x.get("finished").and_then(|f| f.as_bool()) == Some(true)
            {
                return x;
            }
            thread::sleep(Duration::from_millis(10));
        }
        Json::Null
    }

    #[test]
    fn a_job_reports_steps_and_finishes() {
        let j: Arc<Jobs> = Jobs::new();
        let id: u64 = j
            .start(|c| {
                c.begin("one");
                c.ok(50);
                c.begin("two");
                c.mark("unavailable");
                c.finish("unavailable", "later");
            })
            .unwrap();
        let x: Json = wait(&j, id);
        assert_eq!(x.get("state").and_then(|s| s.as_str()), Some("unavailable"));
        assert_eq!(x.get("percent").and_then(|s| s.as_i64()), Some(50));
        assert_eq!(x.get("message").and_then(|s| s.as_str()), Some("later"));
        let steps: &[Json] = x.get("steps").and_then(|s| s.as_arr()).unwrap();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].get("state").and_then(|s| s.as_str()), Some("done"));
        assert_eq!(steps[1].get("state").and_then(|s| s.as_str()), Some("unavailable"));
    }

    #[test]
    fn a_job_that_forgets_to_finish_is_marked_failed() {
        let j: Arc<Jobs> = Jobs::new();
        let id: u64 = j.start(|_| {}).unwrap();
        let x: Json = wait(&j, id);
        assert_eq!(x.get("state").and_then(|s| s.as_str()), Some("failed"));
    }

    #[test]
    fn unknown_ids_are_none() {
        assert!(Jobs::new().get(99).is_none());
    }

    #[test]
    fn running_jobs_are_capped_and_old_ones_trimmed() {
        let j: Arc<Jobs> = Jobs::new();
        let (tx, rx) = mpsc::channel::<()>();
        let rx: Arc<Mutex<mpsc::Receiver<()>>> = Arc::new(Mutex::new(rx));
        let mut ids: Vec<u64> = Vec::new();
        for _ in 0..MAX_RUNNING {
            let r: Arc<Mutex<mpsc::Receiver<()>>> = Arc::clone(&rx);
            ids.push(
                j.start(move |c| {
                    let _ = r.lock().map(|g| g.recv());
                    c.finish("done", "ok");
                })
                .unwrap(),
            );
        }
        assert_eq!(j.running(), MAX_RUNNING);
        assert_eq!(j.start(|c| c.finish("done", "")).unwrap_err().status, 429);
        for _ in 0..MAX_RUNNING {
            tx.send(()).unwrap();
        }
        for id in ids {
            wait(&j, id);
        }
        for _ in 0..(MAX_KEPT + 10) {
            let id: u64 = j.start(|c| c.finish("done", "")).unwrap();
            wait(&j, id);
        }
        assert!(j.lock().len() <= MAX_KEPT);
    }
}
