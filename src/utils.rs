use std::path::PathBuf;

use tiny_skia::Paint;

pub fn lerp<T>(start: T, end: T, t: f32) -> T
where
    T: Copy + std::ops::Add<Output = T> + std::ops::Mul<f32, Output = T>,
{
    start * (1.0 - t) + end * t
}

pub struct Fps {
    last_update: std::time::Instant,
    last: f32,
    update_interval_in_frame: u32,
    frame_count: u32,
}

impl Fps {
    pub fn new(update_interval_in_frame: u32) -> Self {
        Self {
            last_update: std::time::Instant::now(),
            last: 0.0,
            update_interval_in_frame,
            frame_count: 0,
        }
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn update(&mut self) {
        self.frame_count += 1;

        if self.frame_count >= self.update_interval_in_frame {
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(self.last_update).as_secs_f32();

            if elapsed > 0.0 {
                self.last = self.frame_count as f32 / elapsed;
                self.last_update = now;
                self.frame_count = 0;
            }
        }
    }

    pub fn get_fps(&self) -> f32 {
        self.last
    }
}

pub struct TimingCheck {
    time: std::time::Instant,
}

impl TimingCheck {
    pub fn new() -> Self {
        Self {
            time: std::time::Instant::now(),
        }
    }

    pub fn get_time_ns(&self) -> u128 {
        let now = std::time::Instant::now();

        now.duration_since(self.time).as_nanos()
    }
}

pub struct IntervalTimer {
    time_ms: f64,
    interval_ms: f64,
    last_time: std::time::Instant,
}

impl IntervalTimer {
    pub fn new(interval_ms: f64) -> Self {
        Self {
            time_ms: 0.0,
            interval_ms,
            last_time: std::time::Instant::now(),
        }
    }
    
    #[allow(clippy::cast_precision_loss)]
    pub fn update(&mut self) -> bool {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_time).as_millis();
        self.last_time = now;

        self.time_ms += elapsed as f64;

        if self.time_ms < self.interval_ms {
            return false;
        }

        self.time_ms -= self.interval_ms;

        true
    }
}

pub fn default_paint<'a>() -> Paint<'a> {
    Paint::default()
}

pub fn resolve_path(base: &str, target: &str) -> PathBuf {
    let mut path = PathBuf::from(base);

    if path.is_file() {
        path = path.parent().unwrap().to_path_buf();
    }

    path.join(PathBuf::from(target))
}

pub fn exponential_smoothing(current: f32, target: f32, speed: f32, dt: f32) -> f32 {
    current + (target - current) * (1.0 - (-speed * dt).exp())
}

pub struct ExponentialSmoothing {
    current: f32,
    speed: f32,
}

impl ExponentialSmoothing {
    pub fn new(current: f32, speed: f32) -> Self {
        Self { current, speed }
    }

    pub fn get_current(&self) -> f32 {
        self.current
    }

    pub fn update(&mut self, target: f32, dt: f32) -> f32 {
        self.current = exponential_smoothing(self.current, target, self.speed, dt);
        self.current
    }
}

pub struct TimeDelta {
    last_time: std::time::Instant,
}

impl TimeDelta {
    pub fn new() -> Self {
        Self {
            last_time: std::time::Instant::now(),
        }
    }

    pub fn get_without_update_secs(&self) -> f32 {
        self.last_time.elapsed().as_secs_f32()
    }

    pub fn update_and_get_secs(&mut self) -> f32 {
        let now = std::time::Instant::now();
        let delta = now.duration_since(self.last_time).as_secs_f32();
        self.last_time = now;
        delta
    }
}
