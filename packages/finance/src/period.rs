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
}
