use sdk::cosmwasm_std::Timestamp;
use serde::{Deserialize, Serialize};

use crate::duration::Duration;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Period {
    start: Timestamp,
    length: Duration,
}

impl Period {
    pub fn from_till(start: Timestamp, till: Timestamp) -> Self {
        debug_assert!(start <= till);
        Self::from_length(start, Duration::between(start, till))
    }

    pub fn from_length(start: Timestamp, length: Duration) -> Self {
        Self { start, length }
    }

    pub fn start(&self) -> Timestamp {
        self.start
    }

    pub fn length(&self) -> Duration {
        self.length
    }

    pub fn till(&self) -> Timestamp {
        self.start + self.length
    }

    pub fn shift_start(self, delta: Duration) -> Self {
        debug_assert!(delta <= self.length);
        let res = Self::from_length(self.start + delta, self.length - delta);
        debug_assert_eq!(self.till(), res.till());
        res
    }

    pub fn next(self, length: Duration) -> Self {
        Self::from_length(self.till(), length)
    }

    pub fn this(self, length: Duration) -> Self {
        Self::from_length(self.till() - length, length)
    }
}

#[cfg(test)]
mod test {
    use sdk::cosmwasm_std::Timestamp;

    use crate::duration::Duration;

    use super::Period;

    #[test]
    fn shift_start() {
        let p = Period::from_till(
            Timestamp::from_nanos(100),
            Timestamp::from_nanos(100) + Duration::YEAR,
        );
        assert_eq!(
            Period::from_length(
                Timestamp::from_nanos(100) + Duration::HOUR,
                Duration::YEAR - Duration::HOUR
            ),
            p.shift_start(Duration::HOUR)
        );
    }

    #[test]
    fn next() {
        let p = Period::from_length(Timestamp::from_nanos(100), Duration::YEAR);
        assert_eq!(
            Period::from_length(Timestamp::from_nanos(100) + Duration::YEAR, Duration::HOUR),
            p.next(Duration::HOUR)
        );

        assert_eq!(
            p.shift_start(Duration::HOUR).next(Duration::HOUR),
            p.next(Duration::HOUR)
        );
    }

    #[test]
    fn this() {
        let p = Period::from_length(Timestamp::from_nanos(100) + Duration::YEAR, Duration::YEAR);
        assert_eq!(
            Period::from_length(
                Timestamp::from_nanos(100) + Duration::YEAR + Duration::YEAR - Duration::HOUR,
                Duration::HOUR
            ),
            p.this(Duration::HOUR)
        );

        assert_eq!(
            p.this(Duration::HOUR).next(Duration::HOUR),
            p.this(Duration::HOUR)
                .next(Duration::HOUR)
                .this(Duration::HOUR)
        )
    }
}
