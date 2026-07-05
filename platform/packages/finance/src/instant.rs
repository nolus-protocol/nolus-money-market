use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

const NANOS_PER_SECOND: u64 = 1_000_000_000;
const SECONDS_PER_DAY: u64 = 86_400;

/// Wall-clock instant denominated in nanoseconds since the Unix epoch.
///
/// Owned by `finance`. Replaces `cosmwasm_std::Timestamp` for any code path
/// that does not directly meet the cosmwasm API. The serialised form is a
/// stringified `u64` (e.g. `"1234567890"`) — identical to `Timestamp`'s wire
/// shape — so `Item<Timestamp>` storage and inter-contract message fields
/// can be migrated to `Instant` without touching persisted state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant(u64);

impl Instant {
    pub const fn from_nanos(nanos: u64) -> Self {
        Self(nanos)
    }

    pub const fn from_seconds(secs: u64) -> Self {
        Self(secs * NANOS_PER_SECOND)
    }

    pub const fn nanos(&self) -> u64 {
        self.0
    }

    pub const fn seconds(&self) -> u64 {
        self.0 / NANOS_PER_SECOND
    }

    pub const fn plus_nanos(&self, nanos: u64) -> Self {
        Self(self.0 + nanos)
    }

    pub const fn minus_nanos(&self, nanos: u64) -> Self {
        Self(self.0 - nanos)
    }

    pub const fn plus_seconds(&self, secs: u64) -> Self {
        self.plus_nanos(secs * NANOS_PER_SECOND)
    }

    pub const fn plus_days(&self, days: u64) -> Self {
        self.plus_seconds(days * SECONDS_PER_DAY)
    }
}

impl fmt::Display for Instant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{:09}",
            self.0 / NANOS_PER_SECOND,
            self.0 % NANOS_PER_SECOND
        )
    }
}

impl Serialize for Instant {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for Instant {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(InstantVisitor)
    }
}

struct InstantVisitor;

impl<'de> de::Visitor<'de> for InstantVisitor {
    type Value = Instant;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "a u64 nanos value, accepted as either a JSON string or a JSON integer (matching cosmwasm_std::Timestamp / Uint64 leniency)"
        )
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Instant(value))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        value.parse::<u64>().map(Instant).map_err(E::custom)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::Instant;

    #[test]
    fn round_trip_wire_string_pin() {
        let value = Instant::from_nanos(1_234_567_890);
        let encoded = serde_json::to_string(&value).expect("serialize");
        assert_eq!(r#""1234567890""#, encoded);
        let decoded: Instant = serde_json::from_str(&encoded).expect("decode own output");
        assert_eq!(value, decoded);
    }

    #[test]
    fn cross_format_deserialise_string_and_integer() {
        let stringified: Instant =
            serde_json::from_str(r#""1234567890""#).expect("stringified u64 accepted");
        let raw_integer: Instant =
            serde_json::from_str(r#"1234567890"#).expect("raw integer accepted");
        assert_eq!(stringified, raw_integer);
        assert_eq!(Instant::from_nanos(1_234_567_890), stringified);
    }

    #[test]
    fn cosmwasm_timestamp_wire_compatibility() {
        let ts = sdk::cosmwasm_std::Timestamp::from_nanos(7_777_777_777);
        let ts_encoded = serde_json::to_string(&ts).expect("serialize Timestamp");
        let inst: Instant =
            serde_json::from_str(&ts_encoded).expect("decode Timestamp wire bytes as Instant");
        assert_eq!(Instant::from_nanos(7_777_777_777), inst);

        let inst_encoded = serde_json::to_string(&inst).expect("serialize Instant");
        assert_eq!(ts_encoded, inst_encoded);
    }

    #[test]
    fn default_is_epoch_zero() {
        assert_eq!(Instant::from_nanos(0), Instant::default());
        assert_eq!(Instant::from_seconds(0), Instant::default());
        assert_eq!(0, Instant::default().nanos());
    }

    #[test]
    fn from_seconds_matches_from_nanos_times_1e9() {
        assert_eq!(
            Instant::from_nanos(60 * 1_000_000_000),
            Instant::from_seconds(60),
        );
        assert_eq!(60, Instant::from_seconds(60).seconds());
    }

    #[test]
    fn plus_minus_nanos_round_trip() {
        let base = Instant::from_seconds(100);
        assert_eq!(base, base.plus_nanos(42).minus_nanos(42));
        assert_eq!(
            Instant::from_nanos(100 * 1_000_000_000 + 42),
            base.plus_nanos(42),
        );
    }

    #[test]
    fn ordering_is_total_and_consistent() {
        let a = Instant::from_nanos(10);
        let b = Instant::from_nanos(20);
        let c = Instant::from_nanos(30);
        assert!(a < b);
        assert!(b < c);
        assert!(a < c);
        let set: BTreeSet<_> = [c, a, b].into_iter().collect();
        let ordered: Vec<_> = set.iter().copied().collect();
        assert_eq!(vec![a, b, c], ordered);
    }
}
