use std::ops::Add;

use serde::{Deserialize, Serialize};

use sdk::cosmwasm_std::Timestamp;

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

    pub fn till_length(till: Timestamp, max_length: Duration) -> Self {
        let start = if till < Timestamp::default().add(max_length) {
            Timestamp::default()
        } else {
            till - max_length
        };
        Self::from_till(start, till)
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

    // TODO get rid when factor out Period as a result of InterestPeriod::pay
    pub fn shift_start(self, delta: Duration) -> Self {
        debug_assert!(delta <= self.length);
        let res = Self::from_length(self.start + delta, self.length - delta);
        debug_assert_eq!(self.till(), res.till());
        res
    }

    pub fn next(self, length: Duration) -> Self {
        Self::from_length(self.till(), length)
    }

    // TODO remove it
    pub fn this(self, length: Duration) -> Self {
        Self::from_length(self.till() - length, length)
    }

    pub fn intersect(self, other: &Self) -> Self {
        Self::from_till(
            self.move_within(other.start()),
            self.move_within(other.till()),
        )
    }

    pub fn cut_end(self, other: &Self) -> Self {
        debug_assert_eq!(self.intersect(other).till(), self.till());
        Self::from_till(self.start, self.move_within(other.start()))
    }

    // TODO make it private and refactor the callers to use `intersect`
    pub fn move_within(&self, timestamp: Timestamp) -> Timestamp {
        timestamp.clamp(self.start, self.till())
    }
}

#[cfg(test)]
mod test {
    use sdk::cosmwasm_std::Timestamp;

    use crate::duration::Duration;

    use super::Period;

    #[test]
    fn till_length() {
        let p = Period::till_length(Timestamp::from_seconds(1000), Duration::from_secs(200));

        assert_eq!(from_till(800, 1000), p);
    }

    #[test]
    fn till_length_max() {
        let p = Period::till_length(Timestamp::from_seconds(1000), Duration::from_secs(1000));

        assert_eq!(from_till(0, 1000), p);
    }

    #[test]
    fn till_length_underflow() {
        let p = Period::till_length(Timestamp::from_seconds(200), Duration::from_secs(1000));

        assert_eq!(from_till(0, 200), p);
    }

    #[test]
    fn intersect() {
        let p1 = from_till(100, 200);
        assert_eq!(from_till(200, 200), p1.intersect(&from_till(300, 400)));
        assert_eq!(from_till(200, 200), p1.intersect(&from_till(200, 400)));

        assert_eq!(from_till(100, 100), p1.intersect(&from_till(30, 40)));
        assert_eq!(from_till(100, 100), p1.intersect(&from_till(30, 100)));

        assert_eq!(from_till(100, 140), p1.intersect(&from_till(30, 140)));
        assert_eq!(from_till(130, 200), p1.intersect(&from_till(130, 240)));

        assert_eq!(from_till(130, 140), p1.intersect(&from_till(130, 140)));
        assert_eq!(from_till(100, 200), p1.intersect(&from_till(30, 440)));
    }

    #[test]
    fn cut_end() {
        let p1 = from_till(100, 200);
        assert_eq!(p1, p1.cut_end(&from_till(300, 400)));
        assert_eq!(p1, p1.cut_end(&from_till(200, 400)));
        assert_eq!(from_till(100, 180), p1.cut_end(&from_till(180, 400)));
        assert_eq!(from_till(100, 180), p1.cut_end(&from_till(180, 200)));
        assert_eq!(from_till(100, 100), p1.cut_end(&from_till(100, 400)));
        assert_eq!(from_till(100, 100), p1.cut_end(&from_till(80, 200)));
    }

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

    fn from_till(from_sec: u64, till_sec: u64) -> Period {
        Period::from_till(
            Timestamp::from_seconds(from_sec),
            Timestamp::from_seconds(till_sec),
        )
    }
}
