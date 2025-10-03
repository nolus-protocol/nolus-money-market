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
