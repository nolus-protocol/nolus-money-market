use crate::{
    Group, Symbol,
    group::MemberOf,
    matcher,
    visit_any::{AnyVisitor, MatchThenVisit},
};

pub trait GroupVisit
where
    Self: Symbol,
{
    fn maybe_visit_any<V>(symbol: &str, visitor: V) -> Result<V::Outcome, V>
    where
        V: AnyVisitor<Self::Group>,
    {
        let matcher = matcher::symbol_matcher::<Self>(symbol);
        let match_then_visit = MatchThenVisit::new(matcher, visitor);
        <Self::Group as Group>::find_map(match_then_visit).map_err(MatchThenVisit::release_visitor)
    }
}
impl<T> GroupVisit for T
where
    T: Symbol,
    T::Group: MemberOf<T::Group>,
{
}

#[cfg(test)]
mod test {
    use crate::{
        CurrencyDef, Tickers,
        from_symbol_any::GroupVisit,
        test::{
            Expect, ExpectUnknownCurrency, SubGroup, SubGroupTestC10, SuperGroup, SuperGroupTestC1,
            SuperGroupTestC2,
        },
    };

    #[test]
    fn visit_any() {
        let v_usdc = Expect::<SuperGroupTestC1, SuperGroup, SuperGroup>::new();
        assert_eq!(
            Ok(true),
            Tickers::<SuperGroup>::maybe_visit_any(SuperGroupTestC1::ticker(), v_usdc)
        );

        let v_nls = Expect::<SuperGroupTestC2, SuperGroup, SuperGroup>::new();
        assert_eq!(
            Ok(true),
            Tickers::<SuperGroup>::maybe_visit_any(SuperGroupTestC2::ticker(), v_nls)
        );

        let v_err = ExpectUnknownCurrency::<SuperGroup>::new();
        assert_eq!(
            Err(v_err.clone()),
            Tickers::<SuperGroup>::maybe_visit_any(SubGroupTestC10::bank(), v_err)
        );
        let v = ExpectUnknownCurrency::<SuperGroup>::new();
        assert_eq!(
            Err(v.clone()),
            Tickers::<SuperGroup>::maybe_visit_any(SubGroupTestC10::bank(), v)
        );
    }

    #[test]
    fn visit_super_group() {
        assert_eq!(
            Ok(true),
            Tickers::<SuperGroup>::maybe_visit_any(
                SubGroupTestC10::ticker(),
                Expect::<SubGroupTestC10, SuperGroup, SuperGroup>::new()
            )
        );
    }

    #[test]
    fn visit_any_not_in_group() {
        let v_usdc = Expect::<SuperGroupTestC1, SuperGroup, SuperGroup>::new();
        assert_eq!(
            Ok(false),
            Tickers::<SuperGroup>::maybe_visit_any(SubGroupTestC10::ticker(), v_usdc)
        );

        let v_usdc = ExpectUnknownCurrency::<SubGroup>::new();
        assert_eq!(
            Err(v_usdc.clone()),
            Tickers::<SubGroup>::maybe_visit_any(SuperGroupTestC1::ticker(), v_usdc)
        );
    }

    #[test]
    fn visit_any_unexpected() {
        const DENOM: &str = "my_fancy_coin";

        let v_err = ExpectUnknownCurrency::<SuperGroup>::new();
        assert_eq!(
            Err(v_err.clone()),
            Tickers::<SuperGroup>::maybe_visit_any(DENOM, v_err),
        );
    }
}
