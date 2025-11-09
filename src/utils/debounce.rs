use embassy_time::{Duration, Instant};


#[derive(Debug, Copy, Clone)]
pub struct Debounce<T> {
    pub value: T,
    delay: Duration,
    measured_value: T,
    measured_at: Instant,
}

impl<T> Debounce<T> where T: Clone + PartialEq<T> {
    pub fn new(initial: &T, delay: Duration) -> Self {
        Self {
            value: initial.clone(),
            delay,
            measured_value: initial.clone(),
            measured_at: Instant::now(),
        }
    }

    pub fn measure(&mut self, value: &T) -> bool {
        if self.measured_value != *value {
            self.measured_value = value.clone();
            self.measured_at = Instant::now();
        } else if Instant::now().duration_since(self.measured_at) >= self.delay {
            self.measured_value = value.clone();

            return true;
        }

        false
    }
}