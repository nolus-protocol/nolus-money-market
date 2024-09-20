use std::{
    borrow::Borrow,
    collections::BTreeMap,
    hash::{Hash, Hasher},
};

use serde::Deserialize;

use crate::swap_pairs::SwapPairs;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(transparent)]
pub(crate) struct Id(String);

impl Hash for Id {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl Borrow<str> for Id {
    fn borrow(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Deserialize)]
#[serde(transparent)]
pub struct Dexes(BTreeMap<Id, Dex>);

impl Dexes {
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (&str, &Dex)> + '_ {
        self.0.iter().map(|(id, dex)| (id.borrow(), dex))
    }

    #[inline]
    pub fn get<'self_>(&'self_ self, dex: &str) -> Option<&'self_ Dex> {
        self.0.get(dex)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Dex {
    r#type: Type,
    swap_pairs: SwapPairs,
}

impl Dex {
    pub const fn r#type(&self) -> Type {
        self.r#type
    }

    pub const fn swap_pairs(&self) -> &SwapPairs {
        &self.swap_pairs
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum Type {
    AstroportTest,
    AstroportMain,
    Osmosis,
}
