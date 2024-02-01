use std::ops::{Add, Sub};

use serde::{Deserialize, Serialize};

use sdk::cosmwasm_std::Timestamp;

use crate::duration::Duration;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Period {
    start: Timestamp,
    length: Duration,
}

impl Period {
    pub fn from_till(start: Timestamp, till: &Timestamp) -> Self {
        debug_assert!(&start <= till);
        Self::from_length(start, Duration::between(&start, till))
    }

    pub fn from_length(start: Timestamp, length: Duration) -> Self {
        Self { start, length }
    }

    pub fn till_length(till: &Timestamp, max_length: Duration) -> Self {
        let start = if till < &Timestamp::default().add(max_length) {
            Timestamp::default()
        } else {
            till.sub(max_length)
        };
        Self::from_till(start, till)
    }

    pub fn start(&self) -> Timestamp {
        self.start
    }

    pub fn length(&self) -> Duration {
        self.length
    }

    pub fn zero_length(&self) -> bool {
        self.length() == Duration::default()
    }

    pub fn till(&self) -> Timestamp {
        self.start + self.length
    }

    // TODO get rid it out when factor out Period as a result of InterestPeriod::pay
    pub fn shift_start(self, delta: Duration) -> Self {
        debug_assert!(delta <= self.length);
        let res = Self::from_length(self.start + delta, self.length - delta);
        debug_assert_eq!(self.till(), res.till());
        res
    }

    // TODO remove when remove grace time
    pub fn next(self, length: Duration) -> Self {
        Self::from_length(self.till(), length)
    }

    /// Cut off a period from this
    ///
    /// Pre: `self.intersect(other)` either starts at `self.start()` or ends at `self.till()`
    /// Pre: `self.intersect(other) != self`
    pub fn cut(self, other: &Self) -> Self {
        let common = self.intersect(other);
        debug_assert_ne!(common, self);

        let res = if self.start() == common.start() {
            Self::from_till(common.till(), &self.till())
        } else {
            debug_assert_eq!(self.till(), common.till());
            Self::from_till(self.start(), &common.start())
        };

        debug_assert!(common.intersect(&res).zero_length());
        debug_assert_eq!(res.intersect(&self), res);
        debug_assert_eq!(common.intersect(&self), common);
        debug_assert_eq!(self.length, common.length() + res.length());
        res
    }

    // TODO make it private and refactor the callers to use `intersect`
    pub(super) fn move_within(&self, timestamp: Timestamp) -> Timestamp {
        timestamp.clamp(self.start, self.till())
    }

    fn intersect(self, other: &Self) -> Self {
        Self::from_till(
            self.move_within(other.start()),
            &self.move_within(other.till()),
        )
    }
}

#[cfg(test)]
mod test {
    use sdk::cosmwasm_std::Timestamp;

    use crate::duration::Duration;

    use super::Period;

    #[test]
    fn test_from_till() {
        assert!(!from_till(800, 1000).zero_length());
        assert!(from_till(800, 800).zero_length());
    }

    #[test]
    fn till_length() {
        let p = Period::till_length(&Timestamp::from_seconds(1000), Duration::from_secs(200));

        assert_eq!(from_till(800, 1000), p);
        assert!(!p.zero_length());
        assert!(
            Period::till_length(&Timestamp::from_seconds(1000), Duration::default()).zero_length()
        );
    }

    #[test]
    fn till_length_max() {
        let p = Period::till_length(&Timestamp::from_seconds(1000), Duration::from_secs(1000));

        assert_eq!(from_till(0, 1000), p);
        assert!(!p.zero_length());
    }

    #[test]
    fn till_length_underflow() {
        let p = Period::till_length(&Timestamp::from_seconds(200), Duration::from_secs(1000));

        assert_eq!(from_till(0, 200), p);
        assert!(!p.zero_length());
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
    fn cut() {
        let p1 = from_till(100, 200);
        assert_eq!(p1, p1.cut(&from_till(300, 400)));

        assert_eq!(p1, p1.cut(&from_till(80, 100)));
        assert_eq!(p1, p1.cut(&from_till(100, 100)));
        assert_eq!(p1, p1.cut(&from_till(200, 200)));
        assert_eq!(p1, p1.cut(&from_till(200, 400)));

        assert_eq!(from_till(100, 180), p1.cut(&from_till(180, 400)));
        assert_eq!(from_till(100, 180), p1.cut(&from_till(180, 200)));
        assert_eq!(from_till(110, 200), p1.cut(&from_till(80, 110)));
        assert_eq!(from_till(150, 200), p1.cut(&from_till(100, 150)));
    }

    #[test]
    fn shift_start() {
        let p = Period::from_till(
            Timestamp::from_nanos(100),
            &(Timestamp::from_nanos(100) + Duration::YEAR),
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

    fn from_till(from_sec: u64, till_sec: u64) -> Period {
        Period::from_till(
            Timestamp::from_seconds(from_sec),
            &Timestamp::from_seconds(till_sec),
        )
    }
}
