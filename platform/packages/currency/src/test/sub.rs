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
