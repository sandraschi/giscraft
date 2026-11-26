//! Background XYZ tile fetch + PNG decode.
//! The UI thread only enqueues requests and uploads textures that are
//! already decoded. A slow tile server must not stall a frame.

use giscraft_core::tiles::{self, Tile};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub type FetchFn = Arc<dyn Fn(Tile, &str) -> Result<Vec<u8>, String> + Send + Sync>;

#[derive(Debug, Clone)]
struct TileJob {
    generation: u64,
    key: String,
    tile: Tile,
    template: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TileEvent {
    Ready {
        generation: u64,
        key: String,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    },
    Failed {
        generation: u64,
        key: String,
        error: String,
    },
}

impl TileEvent {
    fn generation(&self) -> u64 {
        match self {
            TileEvent::Ready { generation, .. } | TileEvent::Failed { generation, .. } => {
                *generation
            }
        }
    }

    fn key(&self) -> &str {
        match self {
            TileEvent::Ready { key, .. } | TileEvent::Failed { key, .. } => key,
        }
    }
}

/// Cache key shared by the worker and the egui texture map.
pub fn tile_cache_key(template: &str, tile: Tile) -> String {
    format!(
        "{}:{}:{}:{}",
        tiles::template_key(template),
        tile.z,
        tile.x,
        tile.y
    )
}

pub struct TileWorker {
    tx: Sender<TileJob>,
    rx: Receiver<TileEvent>,
    generation: u64,
    pending: HashSet<String>,
    retry_after: HashMap<String, Instant>,
    cooldown: Duration,
    alive: Arc<AtomicBool>,
}

impl TileWorker {
    /// Two workers, 2s pause after a failed tile. Used by the desktop app.
    pub fn spawn() -> Self {
        Self::spawn_with(Arc::new(production_fetch), Duration::from_secs(2), 2)
    }

    pub fn spawn_with(fetch: FetchFn, cooldown: Duration, workers: usize) -> Self {
        let (job_tx, job_rx) = mpsc::channel::<TileJob>();
        let (event_tx, event_rx) = mpsc::channel::<TileEvent>();
        let job_rx = Arc::new(Mutex::new(job_rx));
        let alive = Arc::new(AtomicBool::new(true));
        let n = workers.clamp(1, 4);
        for _ in 0..n {
            let rx = Arc::clone(&job_rx);
            let tx = event_tx.clone();
            let fetch = Arc::clone(&fetch);
            let alive = Arc::clone(&alive);
            std::thread::spawn(move || worker_loop(rx, tx, fetch, alive));
        }
        Self {
            tx: job_tx,
            rx: event_rx,
            generation: 0,
            pending: HashSet::new(),
            retry_after: HashMap::new(),
            cooldown,
            alive,
        }
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Drop queued interest. In-flight jobs still finish, tagged with the
    /// old generation, so the canvas can ignore them.
    pub fn invalidate(&mut self) {
        self.generation = self.generation.saturating_add(1);
        self.pending.clear();
        self.retry_after.clear();
    }

    /// Enqueue one tile. Returns false when it is already queued, cooling
    /// down after a failure, or the queue is at the 64-tile cap.
    pub fn request(&mut self, tile: Tile, template: &str) -> bool {
        let key = tile_cache_key(template, tile);
        if self.pending.contains(&key) || self.pending.len() >= 64 {
            return false;
        }
        if let Some(until) = self.retry_after.get(&key) {
            if Instant::now() < *until {
                return false;
            }
        }
        let job = TileJob {
            generation: self.generation,
            key: key.clone(),
            tile,
            template: template.to_string(),
        };
        match self.tx.send(job) {
            Ok(()) => {
                self.pending.insert(key);
                true
            }
            Err(_) => false,
        }
    }

    pub fn poll(&mut self) -> Vec<TileEvent> {
        let mut out = Vec::new();
        while let Ok(event) = self.rx.try_recv() {
            self.pending.remove(event.key());
            if let TileEvent::Failed { key, .. } = &event {
                if event.generation() == self.generation {
                    self.retry_after
                        .insert(key.clone(), Instant::now() + self.cooldown);
                }
            }
            if let TileEvent::Ready { key, .. } = &event {
                self.retry_after.remove(key);
            }
            out.push(event);
        }
        out
    }
}

impl Drop for TileWorker {
    fn drop(&mut self) {
        self.alive.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

fn production_fetch(tile: Tile, template: &str) -> Result<Vec<u8>, String> {
    tiles::fetch_tile_png_as(tile, template).map_err(|e| e.to_string())
}

fn worker_loop(
    rx: Arc<Mutex<Receiver<TileJob>>>,
    tx: Sender<TileEvent>,
    fetch: FetchFn,
    alive: Arc<AtomicBool>,
) {
    loop {
        if !alive.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }
        let job = {
            let guard = rx.lock().unwrap_or_else(|err| err.into_inner());
            guard.recv()
        };
        let Ok(job) = job else { break };
        if !alive.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }
        let event = decode_job(&fetch, job);
        if tx.send(event).is_err() {
            break;
        }
    }
}

fn decode_job(fetch: &FetchFn, job: TileJob) -> TileEvent {
    match fetch(job.tile, job.template.as_str()) {
        Ok(png) => match tiles::decode_png_rgba(&png) {
            Ok((width, height, rgba)) => TileEvent::Ready {
                generation: job.generation,
                key: job.key,
                width,
                height,
                rgba,
            },
            Err(err) => TileEvent::Failed {
                generation: job.generation,
                key: job.key,
                error: err.to_string(),
            },
        },
        Err(err) => TileEvent::Failed {
            generation: job.generation,
            key: job.key,
            error: err,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;

    /// 1x1 RGBA PNG. Decode must succeed without a network.
    const TINY_PNG: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 10, 73, 68, 65, 84, 120, 156, 99, 0, 1, 0, 0, 5, 0, 1,
        13, 10, 45, 180, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];

    fn sample_tile() -> Tile {
        Tile { z: 3, x: 4, y: 2 }
    }

    fn wait_event(worker: &mut TileWorker) -> Option<TileEvent> {
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            let mut events = worker.poll();
            if let Some(event) = events.pop() {
                return Some(event);
            }
            thread::sleep(Duration::from_millis(5));
        }
        None
    }

    #[test]
    fn request_returns_while_fetch_is_blocked() {
        let release = Arc::new(AtomicBool::new(false));
        let started = Arc::new(AtomicBool::new(false));
        let fetches = Arc::new(AtomicUsize::new(0));
        let release_fetch = Arc::clone(&release);
        let started_fetch = Arc::clone(&started);
        let fetches_fetch = Arc::clone(&fetches);
        let worker = TileWorker::spawn_with(
            Arc::new(move |_tile, _template| {
                fetches_fetch.fetch_add(1, Ordering::SeqCst);
                started_fetch.store(true, Ordering::SeqCst);
                let deadline = Instant::now() + Duration::from_secs(2);
                while !release_fetch.load(Ordering::SeqCst) && Instant::now() < deadline {
                    thread::sleep(Duration::from_millis(5));
                }
                Ok(TINY_PNG.to_vec())
            }),
            Duration::from_secs(2),
            1,
        );
        let mut worker = worker;
        let queued_at = Instant::now();
        assert!(worker.request(sample_tile(), "https://tiles.example.test/{z}/{x}/{y}.png"));
        assert!(queued_at.elapsed() < Duration::from_millis(200));
        let deadline = Instant::now() + Duration::from_secs(2);
        while !started.load(Ordering::SeqCst) && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
        }
        assert!(started.load(Ordering::SeqCst));
        assert!(!worker.request(sample_tile(), "https://tiles.example.test/{z}/{x}/{y}.png"));
        release.store(true, Ordering::SeqCst);
        let event = wait_event(&mut worker);
        assert!(matches!(
            event,
            Some(TileEvent::Ready {
                width: 1,
                height: 1,
                ..
            })
        ));
        assert_eq!(fetches.load(Ordering::SeqCst), 1);
        assert!(!worker.has_pending());
    }

    #[test]
    fn failed_tile_is_not_retried_during_cooldown() {
        let worker = TileWorker::spawn_with(
            Arc::new(|_tile, _template| Err("down".to_string())),
            Duration::from_secs(60),
            1,
        );
        let mut worker = worker;
        assert!(worker.request(sample_tile(), "https://tiles.example.test/{z}/{x}/{y}.png"));
        let event = wait_event(&mut worker);
        assert!(matches!(event, Some(TileEvent::Failed { .. })));
        assert!(!worker.request(sample_tile(), "https://tiles.example.test/{z}/{x}/{y}.png"));
    }

    #[test]
    fn invalidate_marks_inflight_results_stale() {
        let release = Arc::new(AtomicBool::new(false));
        let release_fetch = Arc::clone(&release);
        let worker = TileWorker::spawn_with(
            Arc::new(move |_tile, _template| {
                let deadline = Instant::now() + Duration::from_secs(2);
                while !release_fetch.load(Ordering::SeqCst) && Instant::now() < deadline {
                    thread::sleep(Duration::from_millis(5));
                }
                Ok(TINY_PNG.to_vec())
            }),
            Duration::from_secs(2),
            1,
        );
        let mut worker = worker;
        assert!(worker.request(sample_tile(), "https://tiles.example.test/{z}/{x}/{y}.png"));
        worker.invalidate();
        assert_eq!(worker.generation(), 1);
        assert!(!worker.has_pending());
        release.store(true, Ordering::SeqCst);
        let generation = wait_event(&mut worker).map(|event| event.generation());
        assert_eq!(generation, Some(0));
    }
}
