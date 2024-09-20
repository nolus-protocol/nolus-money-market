use std::{
    borrow::Borrow,
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt::{self, Display, Formatter},
    num::NonZeroUsize,
    ops::Index,
};

use serde::Deserialize;

use crate::currency;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[repr(transparent)]
#[serde(try_from = "BTreeMap<currency::Id, PairTargets>")]
pub struct SwapPairs(BTreeMap<currency::Id, PairTargets>);

impl SwapPairs {
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn contains<Idx>(&self, key: &Idx) -> bool
    where
        currency::Id: Borrow<Idx>,
        Idx: Ord + ?Sized,
    {
        self.0.contains_key(key)
    }

    pub fn get<'r, Idx>(&'r self, key: &Idx) -> Option<&'r PairTargets>
    where
        currency::Id: Borrow<Idx>,
        Idx: Ord + ?Sized,
    {
        self.0.get(key)
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&currency::Id, &PairTargets)> {
        self.0.iter()
    }
}

impl TryFrom<BTreeMap<currency::Id, PairTargets>> for SwapPairs {
    type Error = InPoolWithSelf;

    fn try_from(value: BTreeMap<currency::Id, PairTargets>) -> Result<Self, Self::Error> {
        if value.iter().any(|(from, to)| to.contains(from)) {
            Err(InPoolWithSelf)
        } else {
            Ok(Self(value))
        }
    }
}

impl<Idx> Index<&Idx> for SwapPairs
where
    currency::Id: Borrow<Idx>,
    Idx: Ord + ?Sized,
{
    type Output = PairTargets;

    fn index(&self, index: &Idx) -> &Self::Output {
        &self.0[index]
    }
}

#[derive(Debug, Clone, Copy)]
pub struct InPoolWithSelf;

impl Display for InPoolWithSelf {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("Currency cannot be in pool itself!")
    }
}

impl Error for InPoolWithSelf {}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[repr(transparent)]
#[serde(try_from = "BTreeSet<currency::Id>")]
pub struct PairTargets(BTreeSet<currency::Id>);

impl PairTargets {
    pub fn len(&self) -> NonZeroUsize {
        if let Some(length) = NonZeroUsize::new(self.0.len()) {
            length
        } else {
            unreachable!()
        }
    }

    #[inline]
    pub fn contains<Idx>(&self, index: &Idx) -> bool
    where
        currency::Id: Borrow<Idx>,
        Idx: Ord + ?Sized,
    {
        self.0.contains(index)
    }

    #[inline]
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &currency::Id> {
        self.0.iter()
    }
}

impl TryFrom<BTreeSet<currency::Id>> for PairTargets {
    type Error = NoPairTargets;

    fn try_from(value: BTreeSet<currency::Id>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err(NoPairTargets)
        } else {
            Ok(Self(value))
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NoPairTargets;

impl Display for NoPairTargets {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("No pair targets defined!")
    }
}

impl Error for NoPairTargets {}