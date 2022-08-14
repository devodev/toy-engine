use std::time;

use cgmath::Zero;

#[derive(Debug, PartialEq, PartialOrd)]
pub struct FrameCounter {
    // number of frames is incremented each time on_update() is called.
    frame_count: u64,
    // number of frames since last fps computation.
    fps_frame_count: u64,
    // the fps value computed each time on_update() is called.
    fps: f64,
    // the duration between the last two frames.
    delta_time: time::Duration,
    // the last instant provided to on_update().
    last_time: time::Instant,
}

impl FrameCounter {
    pub fn new() -> Self {
        Self::default()
    }

    /// updates internal state and computes
    pub fn on_update(&mut self, current_time: time::Instant) {
        // compute delta time and set last time to current
        self.delta_time = current_time - self.last_time;
        self.last_time = current_time;

        // increment counters
        self.frame_count += 1;
        self.fps_frame_count += 1;

        // compute fps
        if !self.delta_time.is_zero() {
            self.fps = self.fps_frame_count as f64 / self.delta_time.as_secs_f64();
            self.fps_frame_count = 0;
        }
    }

    #[allow(unused)]
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    pub fn fps(&self) -> f64 {
        self.fps
    }

    pub fn delta_time(&self) -> time::Duration {
        self.delta_time
    }
}

impl Default for FrameCounter {
    fn default() -> Self {
        Self {
            frame_count: 0,
            fps: 0.0,
            fps_frame_count: 0,
            delta_time: time::Duration::ZERO,
            last_time: time::Instant::now(),
        }
    }
}

pub struct FPSPrinter<T: MovingAverage, F: Fn(f64)> {
    throttle_ms: u128,
    delta_time_accumulator: time::Duration,

    moving_average: T,
    print_fn: F,
}

impl<T, F> FPSPrinter<T, F>
where
    T: MovingAverage,
    F: Fn(f64),
{
    pub fn new(moving_average: T, print_fn: F) -> Self {
        Self {
            throttle_ms: 1000,
            delta_time_accumulator: time::Duration::ZERO,
            moving_average,
            print_fn,
        }
    }

    pub fn with_throttle_ms(mut self, throttle_ms: u128) -> Self {
        self.throttle_ms = throttle_ms;
        self
    }

    pub fn on_update(&mut self, delta_time: time::Duration, fps: f64) {
        self.delta_time_accumulator += delta_time;
        // throttle to every second using accumulator
        if self.delta_time_accumulator.as_millis() >= self.throttle_ms {
            // make sure we reset accumulator
            self.delta_time_accumulator = time::Duration::ZERO;
            // compute fps moving average
            let fps_ma = self.moving_average.compute(fps);
            // print fps
            (self.print_fn)(fps_ma);
        }
    }
}

pub trait MovingAverage {
    fn compute(&mut self, fps: f64) -> f64;
}

#[derive(Debug)]
pub struct ExponentialMovingAverage {
    alpha: f64,
    moving_average: f64,
}

impl ExponentialMovingAverage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_alpha(mut self, alpha: f64) -> Self {
        self.alpha = alpha;
        self
    }
}

impl MovingAverage for ExponentialMovingAverage {
    fn compute(&mut self, value: f64) -> f64 {
        if self.moving_average.is_zero() {
            self.moving_average = value;
        }
        self.moving_average = self.alpha * self.moving_average + (1.0 - self.alpha) * value;
        self.moving_average
    }
}

impl Default for ExponentialMovingAverage {
    fn default() -> Self {
        Self {
            alpha: 0.9,
            moving_average: 0.0,
        }
    }
}
