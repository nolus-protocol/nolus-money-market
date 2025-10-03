use currency::{CurrencyDef as _, GroupFindMap, GroupFilterMap};

use self::definitions::{LeaseC1, LeaseC2, LeaseC3, LeaseC4, LeaseC5, LeaseC6, LeaseC7};

use super::Group as LeaseGroup;

pub(super) enum GroupMember {
    LeaseC1,
    LeaseC2,
    LeaseC3,
    LeaseC4,
    LeaseC5,
    LeaseC6,
    LeaseC7,
}

impl currency::GroupMember<super::Group> for GroupMember {
    fn first() -> Option<Self> {
        Some(Self::LeaseC1)
    }

    fn next(&self) -> Option<Self> {
        match self {
            Self::LeaseC1 => Some(Self::LeaseC2),
            Self::LeaseC2 => Some(Self::LeaseC3),
            Self::LeaseC3 => Some(Self::LeaseC4),
            Self::LeaseC4 => Some(Self::LeaseC5),
            Self::LeaseC5 => Some(Self::LeaseC6),
            Self::LeaseC6 => Some(Self::LeaseC7),
            Self::LeaseC7 => None,
        }
    }

    fn filter_map<FilterMap>(&self, filter_map: &FilterMap) -> Option<FilterMap::Outcome>
    where
        FilterMap: GroupFilterMap<VisitedG = super::Group>,
    {
        match self {
            Self::LeaseC1 => filter_map.on::<LeaseC1>(LeaseC1::dto()),
            Self::LeaseC2 => filter_map.on::<LeaseC2>(LeaseC2::dto()),
            Self::LeaseC3 => filter_map.on::<LeaseC3>(LeaseC3::dto()),
            Self::LeaseC4 => filter_map.on::<LeaseC4>(LeaseC4::dto()),
            Self::LeaseC5 => filter_map.on::<LeaseC5>(LeaseC5::dto()),
            Self::LeaseC6 => filter_map.on::<LeaseC6>(LeaseC6::dto()),
            Self::LeaseC7 => filter_map.on::<LeaseC7>(LeaseC7::dto()),
        }
    }

    fn find_map<FindMap>(&self, find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: GroupFindMap<TargetG = super::Group>,
    {
        match self {
            Self::LeaseC1 => find_map.on::<LeaseC1>(LeaseC1::dto()),
            Self::LeaseC2 => find_map.on::<LeaseC2>(LeaseC2::dto()),
            Self::LeaseC3 => find_map.on::<LeaseC3>(LeaseC3::dto()),
            Self::LeaseC4 => find_map.on::<LeaseC4>(LeaseC4::dto()),
            Self::LeaseC5 => find_map.on::<LeaseC5>(LeaseC5::dto()),
            Self::LeaseC6 => find_map.on::<LeaseC6>(LeaseC6::dto()),
            Self::LeaseC7 => find_map.on::<LeaseC7>(LeaseC7::dto()),
        }
    }
}

pub(super) mod definitions {
    use serde::{Deserialize, Serialize};

    use currency::{
        CurrencyDTO, CurrencyDef, Definition, InPoolWith, PairsFindMapT, PairsGroup,
        PairsGroupMember, pairs_find_map,
    };

    use crate::{lpn::Lpn, native::Nls, payment::Group as PaymentGroup};

    use super::LeaseGroup;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    pub struct LeaseC1(CurrencyDTO<LeaseGroup>);

    impl CurrencyDef for LeaseC1 {
        type Group = LeaseGroup;

        #[inline]
        fn dto() -> &'static CurrencyDTO<Self::Group> {
            &const {
                CurrencyDTO::new(
                    const { &Definition::new("LC1", "ibc/bank_LC1", "ibc/dex_LC1", 6) },
                )
            }
        }
    }

    impl PairsGroup for LeaseC1 {
        type CommonGroup = PaymentGroup;

        fn find_map<FindMap>(find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
        where
            FindMap: PairsFindMapT<Pivot = Self>,
        {
            enum Pairs {
                LeaseC2,
                LeaseC3,
            }

            impl PairsGroupMember for Pairs {
                type Group = LeaseC1;

                fn first() -> Option<Self> {
                    Some(Self::LeaseC2)
                }

                fn next(&self) -> Option<Self> {
                    match self {
                        Self::LeaseC2 => Some(Self::LeaseC3),
                        Self::LeaseC3 => None,
                    }
                }

                fn find_map<PairsFindMap>(
                    &self,
                    find_map: PairsFindMap,
                ) -> Result<PairsFindMap::Outcome, PairsFindMap>
                where
                    PairsFindMap: PairsFindMapT<Pivot = Self::Group>,
                {
                    match self {
                        Self::LeaseC2 => find_map.on::<LeaseC2>(LeaseC2::dto()),
                        Self::LeaseC3 => find_map.on::<LeaseC3>(LeaseC3::dto()),
                    }
                }
            }

            pairs_find_map::<Pairs, _>(find_map)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    pub struct LeaseC2(CurrencyDTO<LeaseGroup>);

    impl CurrencyDef for LeaseC2 {
        type Group = LeaseGroup;

        #[inline]
        fn dto() -> &'static CurrencyDTO<Self::Group> {
            &const {
                CurrencyDTO::new(
                    const { &Definition::new("LC2", "ibc/bank_LC2", "ibc/dex_LC2", 6) },
                )
            }
        }
    }

    impl PairsGroup for LeaseC2 {
        type CommonGroup = PaymentGroup;

        fn find_map<FindMap>(find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
        where
            FindMap: PairsFindMapT<Pivot = Self>,
        {
            struct Pairs;

            impl PairsGroupMember for Pairs {
                type Group = LeaseC2;

                fn first() -> Option<Self> {
                    Some(Self)
                }

                fn next(&self) -> Option<Self> {
                    let Self {} = self;

                    None
                }

                fn find_map<PairsFindMap>(
                    &self,
                    find_map: PairsFindMap,
                ) -> Result<PairsFindMap::Outcome, PairsFindMap>
                where
                    PairsFindMap: PairsFindMapT<Pivot = Self::Group>,
                {
                    let Self {} = self;

                    find_map.on::<Lpn>(Lpn::dto())
                }
            }

            pairs_find_map::<Pairs, _>(find_map)
        }
    }

    impl InPoolWith<LeaseC1> for LeaseC2 {}

    impl InPoolWith<LeaseC3> for LeaseC2 {}

    impl InPoolWith<LeaseC4> for LeaseC2 {}

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    pub struct LeaseC3(CurrencyDTO<LeaseGroup>);

    impl CurrencyDef for LeaseC3 {
        type Group = LeaseGroup;

        fn dto() -> &'static CurrencyDTO<Self::Group> {
            &const {
                CurrencyDTO::new(
                    const { &Definition::new("LC3", "ibc/bank_LC3", "ibc/dex_LC3", 6) },
                )
            }
        }
    }

    impl PairsGroup for LeaseC3 {
        type CommonGroup = PaymentGroup;

        fn find_map<FindMap>(find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
        where
            FindMap: PairsFindMapT<Pivot = Self>,
        {
            struct Pairs;

            impl PairsGroupMember for Pairs {
                type Group = LeaseC3;

                fn first() -> Option<Self> {
                    Some(Self)
                }

                fn next(&self) -> Option<Self> {
                    let Self {} = self;

                    None
                }

                fn find_map<PairsFindMap>(
                    &self,
                    find_map: PairsFindMap,
                ) -> Result<PairsFindMap::Outcome, PairsFindMap>
                where
                    PairsFindMap: PairsFindMapT<Pivot = Self::Group>,
                {
                    let Self {} = self;

                    find_map.on::<LeaseC2>(LeaseC2::dto())
                }
            }

            pairs_find_map::<Pairs, _>(find_map)
        }
    }

    impl InPoolWith<LeaseC1> for LeaseC3 {}

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    pub struct LeaseC4(CurrencyDTO<LeaseGroup>);

    impl CurrencyDef for LeaseC4 {
        type Group = LeaseGroup;

        fn dto() -> &'static CurrencyDTO<Self::Group> {
            &const {
                CurrencyDTO::new(
                    const { &Definition::new("LC4", "ibc/bank_LC4", "ibc/dex_LC4", 6) },
                )
            }
        }
    }

    impl PairsGroup for LeaseC4 {
        type CommonGroup = PaymentGroup;

        fn find_map<FindMap>(find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
        where
            FindMap: PairsFindMapT<Pivot = Self>,
        {
            struct Pairs;

            impl PairsGroupMember for Pairs {
                type Group = LeaseC4;

                fn first() -> Option<Self> {
                    Some(Self)
                }

                fn next(&self) -> Option<Self> {
                    let Self {} = self;

                    None
                }

                fn find_map<PairsFindMap>(
                    &self,
                    find_map: PairsFindMap,
                ) -> Result<PairsFindMap::Outcome, PairsFindMap>
                where
                    PairsFindMap: PairsFindMapT<Pivot = Self::Group>,
                {
                    let Self {} = self;

                    find_map.on::<LeaseC2>(LeaseC2::dto())
                }
            }

            pairs_find_map::<Pairs, _>(find_map)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    pub struct LeaseC5(CurrencyDTO<LeaseGroup>);

    impl CurrencyDef for LeaseC5 {
        type Group = LeaseGroup;

        fn dto() -> &'static CurrencyDTO<Self::Group> {
            &const {
                CurrencyDTO::new(
                    const { &Definition::new("LC5", "ibc/bank_LC5", "ibc/dex_LC5", 6) },
                )
            }
        }
    }

    impl PairsGroup for LeaseC5 {
        type CommonGroup = PaymentGroup;

        fn find_map<FindMap>(find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
        where
            FindMap: PairsFindMapT<Pivot = Self>,
        {
            struct Pairs;

            impl PairsGroupMember for Pairs {
                type Group = LeaseC5;

                fn first() -> Option<Self> {
                    Some(Self)
                }

                fn next(&self) -> Option<Self> {
                    let Self {} = self;

                    None
                }

                fn find_map<PairsFindMap>(
                    &self,
                    find_map: PairsFindMap,
                ) -> Result<PairsFindMap::Outcome, PairsFindMap>
                where
                    PairsFindMap: PairsFindMapT<Pivot = Self::Group>,
                {
                    let Self {} = self;

                    find_map.on::<Nls>(Nls::dto())
                }
            }

            pairs_find_map::<Pairs, _>(find_map)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    pub struct LeaseC6(CurrencyDTO<LeaseGroup>);

    impl CurrencyDef for LeaseC6 {
        type Group = LeaseGroup;

        fn dto() -> &'static CurrencyDTO<Self::Group> {
            &const {
                CurrencyDTO::new(
                    const { &Definition::new("LC6", "ibc/bank_LC6", "ibc/dex_LC6", 6) },
                )
            }
        }
    }

    impl PairsGroup for LeaseC6 {
        type CommonGroup = PaymentGroup;

        fn find_map<FindMap>(find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
        where
            FindMap: PairsFindMapT<Pivot = Self>,
        {
            Err(find_map)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    pub struct LeaseC7(CurrencyDTO<LeaseGroup>);

    impl CurrencyDef for LeaseC7 {
        type Group = LeaseGroup;

        fn dto() -> &'static CurrencyDTO<Self::Group> {
            &const {
                CurrencyDTO::new(
                    const { &Definition::new("LC7", "ibc/bank_LC7", "ibc/dex_LC7", 6) },
                )
            }
        }
    }

    impl PairsGroup for LeaseC7 {
        type CommonGroup = PaymentGroup;

        fn find_map<FindMap>(find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
        where
            FindMap: PairsFindMapT<Pivot = Self>,
        {
            struct Pairs;

            impl PairsGroupMember for Pairs {
                type Group = LeaseC7;

                fn first() -> Option<Self> {
                    Some(Self)
                }

                fn next(&self) -> Option<Self> {
                    let Self {} = self;

                    None
                }

                fn find_map<PairsFindMap>(
                    &self,
                    find_map: PairsFindMap,
                ) -> Result<PairsFindMap::Outcome, PairsFindMap>
                where
                    PairsFindMap: PairsFindMapT<Pivot = Self::Group>,
                {
                    let Self {} = self;

                    find_map.on::<Lpn>(Lpn::dto())
                }
            }

            pairs_find_map::<Pairs, _>(find_map)
        }
    }
}

#[cfg(test)]
mod test {
    use currency::CurrencyDef as _;

    use crate::{
        LeaseGroup,
        lpn::{Group as Lpns, Lpn},
        native::Nls,
        test_impl::{
            maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
            maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
        },
    };

    use super::{LeaseC1, LeaseC2, LeaseC3, LeaseC4, LeaseC5, LeaseC6, LeaseC7};

    #[test]
    fn maybe_visit_on_ticker() {
        maybe_visit_on_ticker_impl::<LeaseC1, LeaseGroup>();
        maybe_visit_on_ticker_impl::<LeaseC2, LeaseGroup>();
        maybe_visit_on_ticker_impl::<LeaseC3, LeaseGroup>();
        maybe_visit_on_ticker_impl::<LeaseC4, LeaseGroup>();
        maybe_visit_on_ticker_impl::<LeaseC5, LeaseGroup>();
        maybe_visit_on_ticker_impl::<LeaseC6, LeaseGroup>();
        maybe_visit_on_ticker_impl::<LeaseC7, LeaseGroup>();
        maybe_visit_on_ticker_err::<Lpn, Lpns>(Lpn::bank());
        maybe_visit_on_ticker_err::<LeaseC2, LeaseGroup>(LeaseC2::bank());
        maybe_visit_on_ticker_err::<LeaseC3, LeaseGroup>(LeaseC3::dex());
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<LeaseC1, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<LeaseC2, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<LeaseC3, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<LeaseC4, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<LeaseC5, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<LeaseC6, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<LeaseC7, LeaseGroup>();
        maybe_visit_on_bank_symbol_err::<Lpn, Lpns>(Lpn::ticker());
        maybe_visit_on_bank_symbol_err::<LeaseC1, LeaseGroup>(LeaseC1::ticker());
        maybe_visit_on_bank_symbol_err::<LeaseC1, LeaseGroup>(LeaseC1::dex());
        maybe_visit_on_bank_symbol_err::<LeaseC1, LeaseGroup>(Lpn::ticker());
        maybe_visit_on_bank_symbol_err::<LeaseC1, LeaseGroup>(Nls::bank());
        maybe_visit_on_bank_symbol_err::<LeaseC1, LeaseGroup>(Nls::ticker());
        maybe_visit_on_bank_symbol_err::<LeaseC5, LeaseGroup>(LeaseC5::ticker());
    }
}
