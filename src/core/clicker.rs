use std::collections::VecDeque;
use std::f64::consts::TAU;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use rand::rngs::ThreadRng;
use rand::Rng;
use rand_distr::{Distribution, Normal, Triangular};

use crate::core::input_backend::{create_native_backend, InputBackend, MouseButton};
use crate::core::timing::{HighResolutionTimer, StopSignal};
use crate::core::validation::{MAX_INTERVAL_MS, MIN_INTERVAL_MS};

pub const CORNER_FAILSAFE_PX: i32 = 2;

#[derive(Debug, Clone)]
pub struct ClickSettings {
    pub interval_ms: f64,
    pub button: MouseButton,
    pub double_click: bool,
    pub repeat_count: u64, // 0 = infinite
    pub fixed_position: Option<(i32, i32)>,
    pub humanize: bool,
    pub jitter_pct: f64,
    pub hold_min_ms: f64,
    pub hold_max_ms: f64,
    pub position_jitter_px: i32,
    pub corner_failsafe: bool,
}

impl Default for ClickSettings {
    fn default() -> Self {
        Self {
            interval_ms: 10.0,
            button: MouseButton::Left,
            double_click: false,
            repeat_count: 0,
            fixed_position: None,
            humanize: false,
            jitter_pct: 15.0,
            hold_min_ms: 18.0,
            hold_max_ms: 42.0,
            position_jitter_px: 2,
            corner_failsafe: true,
        }
    }
}

impl ClickSettings {
    pub fn sanitized(&self) -> Self {
        let safe_f = |v: f64, d: f64| if v.is_finite() { v } else { d };
        let mut hold_lo = safe_f(self.hold_min_ms, 18.0).clamp(0.0, 500.0);
        let mut hold_hi = safe_f(self.hold_max_ms, 42.0).clamp(0.0, 500.0);
        if hold_lo > hold_hi {
            std::mem::swap(&mut hold_lo, &mut hold_hi);
        }
        Self {
            interval_ms: safe_f(self.interval_ms, 10.0).clamp(MIN_INTERVAL_MS, MAX_INTERVAL_MS),
            button: self.button,
            double_click: self.double_click,
            repeat_count: self.repeat_count.min(10_000_000),
            fixed_position: self.fixed_position,
            humanize: self.humanize,
            jitter_pct: safe_f(self.jitter_pct, 15.0).clamp(0.0, 80.0),
            hold_min_ms: hold_lo,
            hold_max_ms: hold_hi,
            position_jitter_px: self.position_jitter_px.clamp(0, 50),
            corner_failsafe: self.corner_failsafe,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    Stopped,
    Completed,
    Failsafe,
}

#[derive(Debug, Clone, Copy)]
pub struct EngineSnapshot {
    pub active: bool,
    pub total: u64,
    pub target: u64,
    pub cps: usize,
    pub elapsed_secs: f64,
}

pub type StoppedCallback = Arc<dyn Fn(StopReason) + Send + Sync>;

struct TimingState {
    timestamps: VecDeque<Instant>,
    run_start: Option<Instant>,
    last_elapsed_secs: f64,
}

struct WorkerSlot {
    handle: Option<JoinHandle<()>>,
    stop_signal: Option<Arc<StopSignal>>,
}

pub struct ClickEngine {
    backend: Arc<dyn InputBackend>,
    on_stopped: Option<StoppedCallback>,
    worker: Mutex<WorkerSlot>,
    active: AtomicBool,
    click_count: AtomicU64,
    target_count: AtomicU64,
    held_button: Mutex<Option<MouseButton>>,
    timing: Mutex<TimingState>,
}

impl ClickEngine {
    #[allow(dead_code)]
    pub fn new(on_stopped: Option<StoppedCallback>) -> Self {
        Self::with_backend(Arc::from(create_native_backend()), on_stopped)
    }

    pub fn with_backend(
        backend: Arc<dyn InputBackend>,
        on_stopped: Option<StoppedCallback>,
    ) -> Self {
        Self {
            backend,
            on_stopped,
            worker: Mutex::new(WorkerSlot {
                handle: None,
                stop_signal: None,
            }),
            active: AtomicBool::new(false),
            click_count: AtomicU64::new(0),
            target_count: AtomicU64::new(0),
            held_button: Mutex::new(None),
            timing: Mutex::new(TimingState {
                timestamps: VecDeque::with_capacity(2048),
                run_start: None,
                last_elapsed_secs: 0.0,
            }),
        }
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    pub fn snapshot(&self) -> EngineSnapshot {
        let active = self.is_active();
        let total = self.click_count.load(Ordering::Relaxed);
        let target = self.target_count.load(Ordering::Relaxed);
        let now = Instant::now();
        let cutoff = now.checked_sub(Duration::from_secs(1));

        let mut t = self.timing.lock().unwrap_or_else(|e| e.into_inner());
        if active {
            if let Some(c) = cutoff {
                while let Some(&front) = t.timestamps.front() {
                    if front < c {
                        t.timestamps.pop_front();
                    } else {
                        break;
                    }
                }
            }
        } else {
            t.timestamps.clear();
        }
        let cps = if active { t.timestamps.len() } else { 0 };
        let elapsed_secs = if active {
            t.run_start
                .map(|s| now.saturating_duration_since(s).as_secs_f64())
                .unwrap_or(0.0)
        } else {
            t.last_elapsed_secs
        };
        EngineSnapshot {
            active,
            total,
            target,
            cps,
            elapsed_secs,
        }
    }

    pub fn reset_stats(&self) {
        self.click_count.store(0, Ordering::Relaxed);
        if !self.is_active() {
            self.target_count.store(0, Ordering::Relaxed);
        }
        let mut t = self.timing.lock().unwrap_or_else(|e| e.into_inner());
        t.timestamps.clear();
        t.last_elapsed_secs = 0.0;
        if self.is_active() {
            t.run_start = Some(Instant::now());
        } else {
            t.run_start = None;
        }
    }

    pub fn start(self: &Arc<Self>, settings: ClickSettings) -> bool {
        let mut slot = self.worker.lock().unwrap_or_else(|e| e.into_inner());
        if self.active.load(Ordering::Acquire) {
            return false;
        }
        if let Some(prev) = slot.handle.take() {
            if let Some(sig) = &slot.stop_signal {
                sig.set();
            }
            let _ = prev.join();
        }

        let sanitized = settings.sanitized();
        let stop_signal = Arc::new(StopSignal::new());
        self.target_count
            .store(sanitized.repeat_count, Ordering::Relaxed);
        {
            let mut t = self.timing.lock().unwrap_or_else(|e| e.into_inner());
            t.timestamps.clear();
            t.run_start = Some(Instant::now());
            t.last_elapsed_secs = 0.0;
        }
        self.active.store(true, Ordering::Release);

        let engine = Arc::clone(self);
        let sig_worker = Arc::clone(&stop_signal);
        let handle = thread::Builder::new()
            .name("ClickForgeWorker".to_string())
            .spawn(move || engine.run_worker(sanitized, sig_worker))
            .expect("failed to spawn clicker worker");

        slot.stop_signal = Some(stop_signal);
        slot.handle = Some(handle);
        true
    }

    pub fn stop(&self, wait: bool) {
        let handle = {
            let mut slot = self.worker.lock().unwrap_or_else(|e| e.into_inner());
            self.active.store(false, Ordering::Release);
            if let Some(sig) = &slot.stop_signal {
                sig.set();
            }
            if wait {
                slot.handle.take()
            } else {
                None
            }
        };

        {
            let mut t = self.timing.lock().unwrap_or_else(|e| e.into_inner());
            t.timestamps.clear();
            if let Some(start) = t.run_start {
                t.last_elapsed_secs = Instant::now().saturating_duration_since(start).as_secs_f64();
            }
        }

        if let Some(h) = handle {
            if h.thread().id() != thread::current().id() {
                let _ = h.join();
            }
        }
        self.release_held();
    }

    fn press_button(&self, button: MouseButton) {
        let mut held = self.held_button.lock().unwrap_or_else(|e| e.into_inner());
        *held = Some(button);
        self.backend.press(button);
    }

    pub fn release_held(&self) {
        let mut held = self.held_button.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(btn) = held.take() {
            self.backend.release(btn);
        }
    }

    fn run_worker(&self, s: ClickSettings, stop: Arc<StopSignal>) {
        let _hr_timer = HighResolutionTimer::new(1);
        let mut rng = rand::thread_rng();
        let period_s = s.interval_ms / 1000.0;
        let sigma_s = if s.humanize {
            period_s * s.jitter_pct / 100.0
        } else {
            0.0
        };
        let use_hold = s.humanize && s.hold_max_ms > 0.0;
        let hold_cap_s = period_s * 0.6;
        let clicks_per_action = if s.double_click { 2 } else { 1 };
        let double_gap_s = (period_s * 0.25).min(0.008);

        let corner_margin =
            CORNER_FAILSAFE_PX + if s.humanize { s.position_jitter_px } else { 0 };
        let check_corner = s.corner_failsafe
            && match s.fixed_position {
                None => true,
                Some((fx, fy)) => fx > corner_margin || fy > corner_margin,
            };

        let mut remaining = s.repeat_count;
        let mut jitter_anchor: Option<(i32, i32)> = None;
        let mut last_jitter_pos: Option<(i32, i32)> = None;
        let mut rel_offset: (i32, i32) = (0, 0);
        let mut next_deadline = Instant::now();
        let mut reason = StopReason::Stopped;

        while !stop.is_set() {
            if check_corner && self.in_corner() {
                reason = StopReason::Failsafe;
                break;
            }

            if let Some((fx, fy)) = s.fixed_position {
                let (x, y) = if s.humanize && s.position_jitter_px > 0 {
                    jitter_point(&mut rng, fx, fy, s.position_jitter_px)
                } else {
                    (fx, fy)
                };
                self.backend.move_to(x, y);
            } else if s.humanize && s.position_jitter_px > 0 {
                if let Some(cur) = self.backend.position() {
                    if jitter_anchor.is_none() || Some(cur) != last_jitter_pos {
                        jitter_anchor = Some(cur);
                    }
                    if let Some((ax, ay)) = jitter_anchor {
                        let (jx, jy) = jitter_point(&mut rng, ax, ay, s.position_jitter_px);
                        last_jitter_pos = Some((jx, jy));
                        self.backend.move_to(jx, jy);
                    }
                } else {
                    let (tx, ty) = jitter_point(&mut rng, 0, 0, s.position_jitter_px);
                    let dx = tx - rel_offset.0;
                    let dy = ty - rel_offset.1;
                    rel_offset = (tx, ty);
                    if dx != 0 || dy != 0 {
                        self.backend.move_relative(dx, dy);
                    }
                }
            }

            let mut hold_s = 0.0;
            for idx in 0..clicks_per_action {
                if stop.is_set() {
                    break;
                }
                if use_hold {
                    let sampled_ms = if (s.hold_max_ms - s.hold_min_ms).abs() < 1e-6 {
                        s.hold_min_ms
                    } else {
                        let mode = (s.hold_min_ms + s.hold_max_ms) * 0.5;
                        Triangular::new(s.hold_min_ms, s.hold_max_ms, mode)
                            .map(|dist| dist.sample(&mut rng))
                            .unwrap_or(s.hold_min_ms)
                    };
                    hold_s = (sampled_ms / 1000.0).min(hold_cap_s / clicks_per_action as f64);
                    self.press_button(s.button);
                    stop.wait_for(Duration::from_secs_f64(hold_s.max(0.0)));
                    self.release_held();
                } else {
                    self.backend.click(s.button);
                }

                self.click_count.fetch_add(1, Ordering::Relaxed);
                self.record_timestamp(Instant::now());

                if idx + 1 < clicks_per_action {
                    let gap_s = if use_hold {
                        hold_s.min(hold_cap_s * 0.5)
                    } else {
                        double_gap_s
                    };
                    if !stop.wait_for(Duration::from_secs_f64(gap_s.max(0.0))) {
                        break;
                    }
                }
            }

            if remaining > 0 {
                remaining -= 1;
                if remaining == 0 {
                    reason = StopReason::Completed;
                    break;
                }
            }

            let mut delay_s = period_s;
            if sigma_s > 0.0 {
                if let Ok(normal) = Normal::new(0.0, sigma_s) {
                    let noise = normal.sample(&mut rng).clamp(-2.5 * sigma_s, 2.5 * sigma_s);
                    delay_s = (period_s + noise).max(MIN_INTERVAL_MS / 1000.0);
                }
            }
            next_deadline += Duration::from_secs_f64(delay_s);
            let now = Instant::now();
            if next_deadline + Duration::from_secs_f64(period_s) < now {
                next_deadline = now;
            }
            if !stop.wait_until(next_deadline) {
                break;
            }
        }

        if rel_offset != (0, 0) {
            self.backend.move_relative(-rel_offset.0, -rel_offset.1);
        }
        self.active.store(false, Ordering::Release);
        {
            let mut t = self.timing.lock().unwrap_or_else(|e| e.into_inner());
            t.timestamps.clear();
            if let Some(start) = t.run_start {
                t.last_elapsed_secs = Instant::now().saturating_duration_since(start).as_secs_f64();
            }
        }
        self.release_held();

        if let Some(cb) = &self.on_stopped {
            cb(reason);
        }
    }

    fn record_timestamp(&self, now: Instant) {
        let mut t = self.timing.lock().unwrap_or_else(|e| e.into_inner());
        if t.timestamps.len() >= 8192 {
            t.timestamps.pop_front();
        }
        t.timestamps.push_back(now);
    }

    fn in_corner(&self) -> bool {
        self.backend
            .position()
            .is_some_and(|(x, y)| x <= CORNER_FAILSAFE_PX && y <= CORNER_FAILSAFE_PX)
    }
}

fn jitter_point(rng: &mut ThreadRng, x: i32, y: i32, radius: i32) -> (i32, i32) {
    let r = (radius as f64) * rng.gen::<f64>().sqrt();
    let angle = rng.gen_range(0.0..TAU);
    let dx = (r * angle.cos()).round() as i32;
    let dy = (r * angle.sin()).round() as i32;
    (x + dx, y + dy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::input_backend::{DummyBackend, DummyEvent};
    use std::sync::mpsc;

    #[test]
    fn fixed_repeat_completes_and_resets_cps_on_stop() {
        let backend = Arc::new(DummyBackend::new((300, 300)));
        let (tx, rx) = mpsc::channel();
        let engine = Arc::new(ClickEngine::with_backend(
            backend.clone(),
            Some(Arc::new(move |reason| {
                let _ = tx.send(reason);
            })),
        ));

        let settings = ClickSettings {
            interval_ms: 2.0,
            repeat_count: 5,
            corner_failsafe: false,
            ..Default::default()
        };
        assert!(engine.start(settings));
        let reason = rx.recv_timeout(Duration::from_secs(2)).expect("stopped");
        assert_eq!(reason, StopReason::Completed);
        thread::sleep(Duration::from_millis(15));

        let snap = engine.snapshot();
        assert!(!snap.active);
        assert_eq!(snap.total, 5);
        assert_eq!(snap.cps, 0);
        assert_eq!(
            backend
                .events()
                .iter()
                .filter(|e| matches!(e, DummyEvent::Click(MouseButton::Left)))
                .count(),
            5
        );
    }

    #[test]
    fn anchored_follow_cursor_spread_has_zero_drift() {
        let backend = Arc::new(DummyBackend::new((500, 500)));
        let (tx, rx) = mpsc::channel();
        let engine = Arc::new(ClickEngine::with_backend(
            backend.clone(),
            Some(Arc::new(move |r| {
                let _ = tx.send(r);
            })),
        ));

        let settings = ClickSettings {
            interval_ms: 1.0,
            repeat_count: 25,
            fixed_position: None,
            humanize: true,
            jitter_pct: 0.0,
            hold_min_ms: 0.0,
            hold_max_ms: 0.0,
            position_jitter_px: 4,
            corner_failsafe: false,
            ..Default::default()
        };
        assert!(engine.start(settings));
        rx.recv_timeout(Duration::from_secs(2)).unwrap();

        for ev in backend.events() {
            if let DummyEvent::Move(x, y) = ev {
                assert!(
                    (x - 500).abs() <= 5 && (y - 500).abs() <= 5,
                    "drifted too far: ({x}, {y})"
                );
            }
        }
    }

    #[test]
    fn corner_failsafe_stops_run() {
        let backend = Arc::new(DummyBackend::new((0, 0)));
        let (tx, rx) = mpsc::channel();
        let engine = Arc::new(ClickEngine::with_backend(
            backend,
            Some(Arc::new(move |r| {
                let _ = tx.send(r);
            })),
        ));
        let settings = ClickSettings {
            interval_ms: 5.0,
            corner_failsafe: true,
            ..Default::default()
        };
        assert!(engine.start(settings));
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            StopReason::Failsafe
        );
    }

    #[test]
    fn guaranteed_mouse_release_when_stopped_mid_hold() {
        let backend = Arc::new(DummyBackend::new((400, 400)));
        let engine = Arc::new(ClickEngine::with_backend(backend.clone(), None));
        let settings = ClickSettings {
            interval_ms: 500.0,
            humanize: true,
            hold_min_ms: 200.0,
            hold_max_ms: 200.0,
            corner_failsafe: false,
            ..Default::default()
        };
        assert!(engine.start(settings));
        thread::sleep(Duration::from_millis(30));
        engine.stop(true);
        let events = backend.events();
        let presses = events
            .iter()
            .filter(|e| matches!(e, DummyEvent::Press(_)))
            .count();
        let releases = events
            .iter()
            .filter(|e| matches!(e, DummyEvent::Release(_)))
            .count();
        assert_eq!(presses, releases);
    }

    #[test]
    fn relative_follow_cursor_spread_has_zero_drift() {
        let backend = Arc::new(DummyBackend::with_unknown_pos());
        let (tx, rx) = mpsc::channel();
        let engine = Arc::new(ClickEngine::with_backend(
            backend.clone(),
            Some(Arc::new(move |r| {
                let _ = tx.send(r);
            })),
        ));
        let settings = ClickSettings {
            interval_ms: 1.0,
            repeat_count: 15,
            fixed_position: None,
            humanize: true,
            jitter_pct: 0.0,
            hold_min_ms: 0.0,
            hold_max_ms: 0.0,
            position_jitter_px: 5,
            corner_failsafe: false,
            ..Default::default()
        };
        assert!(engine.start(settings));
        rx.recv_timeout(Duration::from_secs(2)).unwrap();

        let (sum_dx, sum_dy) = backend.events().into_iter().fold((0, 0), |(sx, sy), ev| {
            if let DummyEvent::MoveRel(dx, dy) = ev {
                (sx + dx, sy + dy)
            } else {
                (sx, sy)
            }
        });
        assert_eq!((sum_dx, sum_dy), (0, 0));
    }
}
