use tiny_skia::Paint;

pub fn lerp<T>(start: T, end: T, t: f32) -> T
where
    T: Copy + std::ops::Add<Output = T> + std::ops::Mul<f32, Output = T>,
{
    start * (1.0 - t) + end * t
}

pub struct Fps {
    last_update: std::time::Instant,
    last_fps: u32,
    update_interval_in_frame: u32,
    frame_count: u32,
}

impl Fps {
    pub fn new(update_interval_in_frame: u32) -> Self {
        Self {
            last_update: std::time::Instant::now(),
            last_fps: 0,
            update_interval_in_frame,
            frame_count: 0,
        }
    }

    pub fn update(&mut self) {
        self.frame_count += 1;

        if self.frame_count >= self.update_interval_in_frame {
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(self.last_update).as_secs_f32();

            if elapsed > 0.0 {
                self.last_fps = (self.frame_count as f32 / elapsed) as u32;
                self.last_update = now;
                self.frame_count = 0;
            }
        }
    }

    pub fn get_fps(&self) -> u32 {
        self.last_fps
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
