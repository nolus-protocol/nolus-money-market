use crate::{
    CurrencyDef, FindMapT,
    group::{FilterMapT, GroupMember},
    test::{SubGroup, SubGroupTestC6, SubGroupTestC10},
};

// ======== START GENERATED CODE =========
pub(super) enum Item {
    SubGroupTestC6(),
    SubGroupTestC10(),
}

impl GroupMember<SubGroup> for Item {
    fn first() -> Option<Self> {
        Some(Self::SubGroupTestC6())
    }

    fn next(&self) -> Option<Self> {
        match self {
            Item::SubGroupTestC6() => Some(Self::SubGroupTestC10()),
            Item::SubGroupTestC10() => None,
        }
    }

    fn filter_map<FilterMap>(&self, filter_map: &FilterMap) -> Option<FilterMap::Outcome>
    where
        FilterMap: FilterMapT<VisitedG = SubGroup>,
    {
        match *self {
            Item::SubGroupTestC6() => filter_map.on::<SubGroupTestC6>(SubGroupTestC6::dto()),
            Item::SubGroupTestC10() => filter_map.on::<SubGroupTestC10>(SubGroupTestC10::dto()),
        }
    }

    fn find_map<FindMap>(&self, find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: FindMapT<TargetG = SubGroup>,
    {
        match *self {
            Item::SubGroupTestC6() => find_map.on::<SubGroupTestC6>(SubGroupTestC6::dto()),
            Item::SubGroupTestC10() => find_map.on::<SubGroupTestC10>(SubGroupTestC10::dto()),
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
            FindCurrencyBySymbol, SubGroup, SubGroupTestC6, SubGroupTestC10, SuperGroupTestC1,
            filter::{Dto, FindByTicker},
        },
    };

    #[test]
    fn enumerate_all() {
        let filter = Dto::<SubGroup>::default();
        let mut iter = SubGroup::filter_map::<Dto<SubGroup>, _>(&filter);
        assert_eq!(Some(SubGroupTestC6::dto()), iter.next().as_ref());
        assert_eq!(Some(SubGroupTestC10::dto()), iter.next().as_ref());
        assert_eq!(None, iter.next().as_ref());
    }

    #[test]
    fn skip_some() {
        let filter = FindByTicker::new(SubGroupTestC10::ticker(), SuperGroupTestC1::ticker());
        let mut iter = SubGroup::filter_map(filter);
        assert_eq!(Some(SubGroupTestC10::dto()), iter.next().as_ref());
        assert_eq!(None, iter.next().as_ref());
    }

    #[test]
    fn find() {
        find_ok::<SubGroupTestC6>();
        find_ok::<SubGroupTestC10>();
        find_nok("unknown ticker");
    }

    #[track_caller]
    fn find_ok<C>()
    where
        C: CurrencyDef,
    {
        let matcher = symbol_matcher::<Tickers<SubGroup>>(C::ticker());
        assert_eq!(
            C::dto(),
            &SubGroup::find_map(FindCurrencyBySymbol::with_matcher(matcher)).unwrap()
        );
    }

    fn find_nok(ticker: &str) {
        let matcher = symbol_matcher::<Tickers<SubGroup>>(ticker);
        assert!(SubGroup::find_map(FindCurrencyBySymbol::with_matcher(matcher)).is_err());
    }
}
