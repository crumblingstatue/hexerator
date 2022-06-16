use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct Timer {
    init_point: Instant,
    duration: Duration,
}

impl Timer {
    pub fn set(duration: Duration) -> Self {
        Self {
            init_point: Instant::now(),
            duration,
        }
    }
    pub fn overtime(&self) -> Option<Duration> {
        let elapsed = self.init_point.elapsed();
        if elapsed > self.duration {
            None
        } else {
            Some(elapsed)
        }
    }
}

impl Default for Timer {
    fn default() -> Self {
        Timer::set(Duration::ZERO)
    }
}
