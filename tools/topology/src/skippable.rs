use serde::{Deserialize, Deserializer};

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
