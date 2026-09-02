use std::time::{Duration, Instant};

#[derive(Default)]
pub struct Profile {
    pub regex: Duration,
    pub bytes: Duration,
    pub cache: Duration,
    pub merge: Duration,
    pub output: Duration,
}

pub fn time<T>(total: &mut Duration, work: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let value = work();
    *total += start.elapsed();
    value
}
