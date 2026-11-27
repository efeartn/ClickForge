use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

const SPIN_THRESHOLD: Duration = Duration::from_micros(1500);

/// RAII guard raising the OS scheduler resolution to 1 ms on Windows.
pub struct HighResolutionTimer {
    #[cfg(windows)]
    period: u32,
    #[cfg(windows)]
    active: bool,
}

impl HighResolutionTimer {
    pub fn new(period_ms: u32) -> Self {
        #[cfg(windows)]
        {
            let ok = unsafe { windows_sys::Win32::Media::timeBeginPeriod(period_ms) == 0 };
            return Self {
                period: period_ms,
                active: ok,
            };
        }
        #[cfg(not(windows))]
        {
            let _ = period_ms;
            Self {}
        }
    }
}

impl Drop for HighResolutionTimer {
    fn drop(&mut self) {
        #[cfg(windows)]
        if self.active {
            unsafe {
                windows_sys::Win32::Media::timeEndPeriod(self.period);
            }
            self.active = false;
        }
    }
}

pub struct StopSignal {
    lock: Mutex<bool>,
    cv: Condvar,
}

impl StopSignal {
    pub fn new() -> Self {
        Self {
            lock: Mutex::new(false),
            cv: Condvar::new(),
        }
    }

    pub fn is_set(&self) -> bool {
        *self.lock.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn set(&self) {
        let mut guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        *guard = true;
        self.cv.notify_all();
    }

    pub fn wait_until(&self, deadline: Instant) -> bool {
        loop {
            let now = Instant::now();
            if now >= deadline {
                return !self.is_set();
            }
            let remaining = deadline.saturating_duration_since(now);
            if remaining > SPIN_THRESHOLD {
                let sleep_dur = remaining - SPIN_THRESHOLD;
                let guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
                if *guard {
                    return false;
                }
                let (guard, _) = self
                    .cv
                    .wait_timeout(guard, sleep_dur)
                    .unwrap_or_else(|e| e.into_inner());
                if *guard {
                    return false;
                }
                continue;
            }
            while Instant::now() < deadline {
                if self.is_set() {
                    return false;
                }
                std::hint::spin_loop();
            }
            return !self.is_set();
        }
    }

    pub fn wait_for(&self, duration: Duration) -> bool {
        if duration.is_zero() {
            return !self.is_set();
        }
        self.wait_until(Instant::now() + duration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn stop_signal_interrupts_wait_early() {
        let sig = Arc::new(StopSignal::new());
        let sig2 = Arc::clone(&sig);
        let start = Instant::now();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            sig2.set();
        });
        let completed = sig.wait_for(Duration::from_secs(2));
        assert!(!completed);
        assert!(start.elapsed() < Duration::from_millis(500));
    }

    #[test]
    fn high_res_timer_guard_constructs() {
        let _guard = HighResolutionTimer::new(1);
    }
}
