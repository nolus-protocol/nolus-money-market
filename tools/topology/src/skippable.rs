use serde::{Deserialize, Deserializer};

/// Type for use when being able to have a field missing during
/// deserialization, but not allowed to hold a nil value, e.g. JSON's `null`.
///
/// # Example
/// ```ignore
/// #[derive(serde::Deserialize)]
/// struct Object {
///   // Allowed values: 0..255
///   required: u8,
///   // Allowed values: 0..255
///   // Note: acts like wrapped type when without `#[serde(default)]`.
///   required: Skippable<u8>,
///   // Allowed values: null, 0..255
///   required_nullable: Option<u8>,
///   // Allowed values: unset, 0..255
///   #[serde(default)]
///   skippable: Skippable<u8>,
///   // Allowed values: unset, null, 0..255
///   #[serde(default)]
///   skippable_nullable: Option<u8>,
/// }
/// ```
// Current uses of the type are fields akin to `icon` and `override_symbol`,
// which aren't strictly required to be present but should not allow null as an
// allowed value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub(crate) enum Skippable<T> {
    #[default]
    Skipped,
    Some(T),
}

impl<'r, T> Deserialize<'r> for Skippable<T>
where
    T: Deserialize<'r>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'r>,
    {
        T::deserialize(deserializer).map(Self::Some)
    }
}
