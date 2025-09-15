use crate::{
    CurrencyDef, FindMapT,
    group::{FilterMapT, GroupMember},
    test::{
        SuperGroup, SuperGroupTestC1, SuperGroupTestC2, SuperGroupTestC3, SuperGroupTestC4,
        SuperGroupTestC5,
    },
};

// ======== START GENERATED CODE =========
pub(super) enum Item {
    SuperGroupTestC1(),
    SuperGroupTestC2(),
    SuperGroupTestC3(),
    SuperGroupTestC4(),
    SuperGroupTestC5(),
}

impl GroupMember<SuperGroup> for Item {
    fn first() -> Option<Self> {
        Some(Self::SuperGroupTestC1())
    }

    fn next(&self) -> Option<Self> {
        match self {
            Item::SuperGroupTestC1() => Some(Self::SuperGroupTestC2()),
            Item::SuperGroupTestC2() => Some(Self::SuperGroupTestC3()),
            Item::SuperGroupTestC3() => Some(Self::SuperGroupTestC4()),
            Item::SuperGroupTestC4() => Some(Self::SuperGroupTestC5()),
            Item::SuperGroupTestC5() => None,
        }
    }

    fn filter_map<FilterMap>(&self, filter_map: &FilterMap) -> Option<FilterMap::Outcome>
    where
        FilterMap: FilterMapT<SuperGroup>,
    {
        match *self {
            Item::SuperGroupTestC1() => filter_map.on::<SuperGroupTestC1>(SuperGroupTestC1::dto()),
            Item::SuperGroupTestC2() => filter_map.on::<SuperGroupTestC2>(SuperGroupTestC2::dto()),
            Item::SuperGroupTestC3() => filter_map.on::<SuperGroupTestC3>(SuperGroupTestC3::dto()),
            Item::SuperGroupTestC4() => filter_map.on::<SuperGroupTestC4>(SuperGroupTestC4::dto()),
            Item::SuperGroupTestC5() => filter_map.on::<SuperGroupTestC5>(SuperGroupTestC5::dto()),
        }
    }

    fn find_map<FindMap>(&self, find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: FindMapT<TargetG = SuperGroup>,
    {
        match *self {
            Item::SuperGroupTestC1() => find_map.on::<SuperGroupTestC1>(SuperGroupTestC1::dto()),
            Item::SuperGroupTestC2() => find_map.on::<SuperGroupTestC2>(SuperGroupTestC2::dto()),
            Item::SuperGroupTestC3() => find_map.on::<SuperGroupTestC3>(SuperGroupTestC3::dto()),
            Item::SuperGroupTestC4() => find_map.on::<SuperGroupTestC4>(SuperGroupTestC4::dto()),
            Item::SuperGroupTestC5() => find_map.on::<SuperGroupTestC5>(SuperGroupTestC5::dto()),
        }
    }
}

// ======== END GENERATED CODE =========

#[cfg(test)]
mod test {

    use crate::{
        CurrencyDef, Group, Tickers,
        matcher::symbol_matcher,
        test::{
            FindCurrencyBySymbol, SubGroupTestC6, SubGroupTestC10, SuperGroup, SuperGroupTestC1,
            SuperGroupTestC2, SuperGroupTestC3, SuperGroupTestC4, SuperGroupTestC5,
            filter::{Dto, FindByTicker},
        },
    };

    #[test]
    fn enumerate_all() {
        let filter = Dto::default();
        let mut iter = SuperGroup::filter_map(filter);

        assert_eq!(Some(SuperGroupTestC1::dto()), iter.next().as_ref());
        assert_eq!(Some(SuperGroupTestC2::dto()), iter.next().as_ref());
        assert_eq!(Some(SuperGroupTestC3::dto()), iter.next().as_ref());
        assert_eq!(Some(SuperGroupTestC4::dto()), iter.next().as_ref());
        assert_eq!(Some(SuperGroupTestC5::dto()), iter.next().as_ref());
        assert_eq!(Some(SubGroupTestC6::dto().into_super_group()), iter.next());
        assert_eq!(Some(SubGroupTestC10::dto().into_super_group()), iter.next());
        assert_eq!(None, iter.next().as_ref());
    }

    #[test]
    fn skip_some() {
        let filter = FindByTicker::new(SuperGroupTestC3::ticker(), SubGroupTestC10::ticker());
        let mut iter = SuperGroup::filter_map(filter);
        assert_eq!(Some(SuperGroupTestC3::dto()), iter.next().as_ref());
        assert_eq!(Some(SubGroupTestC10::dto().into_super_group()), iter.next());
        assert_eq!(None, iter.next().as_ref());
    }

    #[test]
    fn find() {
        find_ok::<SuperGroupTestC1>();
        find_ok::<SuperGroupTestC2>();
        find_ok::<SuperGroupTestC3>();
        find_ok::<SuperGroupTestC4>();
        find_ok::<SuperGroupTestC5>();
        find_ok::<SubGroupTestC6>();
        find_ok::<SubGroupTestC10>();
        find_nok("unknown ticker");
    }

    #[track_caller]
    fn find_ok<C>()
    where
        C: CurrencyDef,
    {
        let matcher = symbol_matcher::<Tickers<SuperGroup>>(C::ticker());
        assert_eq!(
            C::dto(),
            &SuperGroup::find_map(FindCurrencyBySymbol::with_matcher(matcher)).unwrap()
        );
    }

    fn find_nok(ticker: &str) {
        let matcher = symbol_matcher::<Tickers<SuperGroup>>(ticker);
        assert!(SuperGroup::find_map(FindCurrencyBySymbol::with_matcher(matcher)).is_err());
    }
}
