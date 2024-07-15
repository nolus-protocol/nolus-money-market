use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ComparedEntry<Key, AssociatedValue, const ALLOW_EQUAL: bool, const MAXIMUM: bool>
{
    key: Key,
    associated_value: AssociatedValue,
}

impl<Key, AssociatedValue, const ALLOW_EQUAL: bool, const MAXIMUM: bool>
    ComparedEntry<Key, AssociatedValue, ALLOW_EQUAL, MAXIMUM>
{
    pub const fn key(&self) -> Key
    where
        Key: Copy,
    {
        self.key
    }

    pub const fn associated_value(&self) -> &AssociatedValue {
        &self.associated_value
    }

    pub fn into_key_value(self) -> (Key, AssociatedValue) {
        (self.key, self.associated_value)
    }
}

pub(crate) type Min<Key, AssociatedValue, const ALLOW_EQUAL: bool> =
    ComparedEntry<Key, AssociatedValue, ALLOW_EQUAL, false>;

pub(crate) type Max<Key, AssociatedValue, const ALLOW_EQUAL: bool> =
    ComparedEntry<Key, AssociatedValue, ALLOW_EQUAL, true>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ComparedPair<Key, AssociatedValue, const ALLOW_EQUAL: bool> {
    min: Min<Key, AssociatedValue, ALLOW_EQUAL>,
    max: Max<Key, AssociatedValue, ALLOW_EQUAL>,
}

impl<Key, AssociatedValue> ComparedPair<Key, AssociatedValue, false>
where
    Key: Ord,
{
    #[inline]
    pub fn new(left: (Key, AssociatedValue), right: (Key, AssociatedValue)) -> Option<Self> {
        Self::with_swap_status(left, right).map(
            #[inline]
            |(pair, _)| pair,
        )
    }

    pub fn with_swap_status(
        left: (Key, AssociatedValue),
        right: (Key, AssociatedValue),
    ) -> Option<(Self, SwapStatus)> {
        match left.0.cmp(&right.0) {
            Ordering::Less => Some((
                Self {
                    min: Min {
                        key: left.0,
                        associated_value: left.1,
                    },
                    max: Max {
                        key: right.0,
                        associated_value: right.1,
                    },
                },
                SwapStatus::NotSwapped,
            )),
            Ordering::Equal => None,
            Ordering::Greater => Some((
                Self {
                    min: Min {
                        key: right.0,
                        associated_value: right.1,
                    },
                    max: Max {
                        key: left.0,
                        associated_value: left.1,
                    },
                },
                SwapStatus::Swapped,
            )),
        }
    }
}

impl<Key, AssociatedValue, const ALLOW_EQUAL: bool>
    ComparedPair<Key, AssociatedValue, ALLOW_EQUAL>
{
    pub const fn min(&self) -> &Min<Key, AssociatedValue, ALLOW_EQUAL> {
        &self.min
    }

    pub const fn max(&self) -> &Max<Key, AssociatedValue, ALLOW_EQUAL> {
        &self.max
    }

    pub fn map_associated_values<
        NewAssociatedValue,
        F: FnMut(AssociatedValue) -> NewAssociatedValue,
    >(
        self,
        mut f: F,
    ) -> ComparedPair<Key, NewAssociatedValue, ALLOW_EQUAL> {
        ComparedPair {
            min: Min {
                key: self.min.key,
                associated_value: f(self.min.associated_value),
            },
            max: Max {
                key: self.max.key,
                associated_value: f(self.max.associated_value),
            },
        }
    }

    pub fn map_associated_values_detached<
        NewAssociatedValue,
        MinF: FnOnce(AssociatedValue) -> NewAssociatedValue,
        MaxF: FnOnce(AssociatedValue) -> NewAssociatedValue,
    >(
        self,
        min_f: MinF,
        max_f: MaxF,
    ) -> ComparedPair<Key, NewAssociatedValue, ALLOW_EQUAL> {
        ComparedPair {
            min: Min {
                key: self.min.key,
                associated_value: min_f(self.min.associated_value),
            },
            max: Max {
                key: self.max.key,
                associated_value: max_f(self.max.associated_value),
            },
        }
    }

    pub fn take_associated_values(
        self,
    ) -> (
        ComparedPair<Key, (), ALLOW_EQUAL>,
        (AssociatedValue, AssociatedValue),
    ) {
        (
            ComparedPair {
                min: Min {
                    key: self.min.key,
                    associated_value: (),
                },
                max: Max {
                    key: self.max.key,
                    associated_value: (),
                },
            },
            (self.min.associated_value, self.max.associated_value),
        )
    }

    pub fn into_entries(
        self,
    ) -> (
        Min<Key, AssociatedValue, ALLOW_EQUAL>,
        Max<Key, AssociatedValue, ALLOW_EQUAL>,
    ) {
        (self.min, self.max)
    }
}

pub(crate) type ComparedPairNotEq<Key, AssociatedValue> = ComparedPair<Key, AssociatedValue, false>;

pub(crate) enum SwapStatus {
    NotSwapped,
    Swapped,
}
