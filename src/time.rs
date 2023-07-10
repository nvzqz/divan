use std::{fmt, time::Duration};

/// [Picosecond](https://en.wikipedia.org/wiki/Picosecond)-precise [`Duration`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SmallDuration {
    pub picos: u128,
}

impl fmt::Debug for SmallDuration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // `Duration` has no notion of picoseconds, so we manually format
        // picoseconds and nanoseconds ourselves.
        if self.picos < 1_000 {
            write!(f, "{}ps", self.picos)
        } else if self.picos < 1_000_000 {
            let nanos = self.picos as f64 / 1_000.0;
            write!(f, "{}ns", nanos)
        } else {
            Duration::from_nanos((self.picos / 1_000) as u64).fmt(f)
        }
    }
}

impl SmallDuration {
    /// Computes the average of a duration over a number of elements.
    pub fn average(duration: Duration, n: u128) -> Self {
        Self { picos: (duration.as_nanos() * 1_000) / n }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_duration_average() {
        _ = SmallDuration::average(Duration::MAX, 1);
    }
}
