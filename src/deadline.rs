use std::time::Instant;

#[derive(Clone)]
pub struct Deadline {
    pub t_s: Instant,
    pub time_limit_ms: f64,
}

impl Deadline {
    pub fn new(time_limit_ms: f64) -> Self {
        Self {
            t_s: Instant::now(),
            time_limit_ms,
        }
    }

    pub fn elapsed_ms(&self) -> f64 {
        return self.t_s.elapsed().as_secs_f64() * 1000.0;
    }

    pub fn is_expired(&self) -> bool {
        return self.elapsed_ms() > self.time_limit_ms;
    }
}
