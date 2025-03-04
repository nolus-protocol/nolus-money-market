/// This module implements (de-)serialation of Amount as String
/// to keep compatibility with pre-CW 2.x
use std::fmt::Formatter;

use serde::{
    Deserializer, Serializer,
    de::{Unexpected, Visitor},
};

use crate::coin::Amount;

pub(super) fn serialize<S>(amount: &Amount, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&amount.to_string())
}

pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<Amount, D::Error>
where
    D: Deserializer<'de>,
{
    struct StrVisitor;
    impl Visitor<'_> for StrVisitor {
        type Value = Amount;

        fn expecting(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("\"<u128>\"")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            v.parse()
                .map_err(|_| E::invalid_value(Unexpected::Str(v), &self))
        }
    }

    deserializer.deserialize_str(StrVisitor)
}
