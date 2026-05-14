//! Bridge between `finance::Instant` (the project-owned time type) and
//! `cosmwasm_std::Timestamp` (the cosmwasm-API time type).
//!
//! `finance` no longer depends on `cosmwasm-std` so it cannot define
//! conversions to / from `Timestamp` directly without re-introducing the
//! dependency. The orphan rule also prevents writing the `From` impls in any
//! third crate that owns neither type. Two extension traits — owned by this
//! crate — fill the gap.
//!
//! Import them where the cosmwasm boundary meets finance time arithmetic:
//!
//! ```ignore
//! use cw_time::{IntoInstant, IntoTimestamp};
//!
//! let now = env.block.time.into_instant();          // boundary in
//! let ibc_timeout = (now + duration).into_timestamp(); // boundary out
//! ```

use finance::instant::Instant;
use sdk::cosmwasm_std::Timestamp;

pub trait IntoInstant {
    fn into_instant(self) -> Instant;
}

impl IntoInstant for Timestamp {
    fn into_instant(self) -> Instant {
        Instant::from_nanos(self.nanos())
    }
}

pub trait IntoTimestamp {
    fn into_timestamp(self) -> Timestamp;
}

impl IntoTimestamp for Instant {
    fn into_timestamp(self) -> Timestamp {
        Timestamp::from_nanos(self.nanos())
    }
}

#[cfg(test)]
mod tests {
    use finance::instant::Instant;
    use sdk::cosmwasm_std::Timestamp;

    use super::{IntoInstant, IntoTimestamp};

    #[test]
    fn timestamp_round_trip() {
        let ts = Timestamp::from_nanos(1_234_567_890);
        let back = ts.into_instant().into_timestamp();
        assert_eq!(ts, back);
    }

    #[test]
    fn instant_round_trip() {
        let inst = Instant::from_nanos(7_777_777_777);
        let back = inst.into_timestamp().into_instant();
        assert_eq!(inst, back);
    }

    #[test]
    fn both_directions_preserve_nanos() {
        let ts = Timestamp::from_seconds(60);
        assert_eq!(60 * 1_000_000_000, ts.into_instant().nanos());

        let inst = Instant::from_seconds(60);
        assert_eq!(60 * 1_000_000_000, inst.into_timestamp().nanos());
    }
}
