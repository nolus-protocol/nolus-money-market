use std::marker::PhantomData;

use currency::{CurrencyDTO, Group, MemberOf};
use finance::{
    coin::{Amount, CoinDTO},
    zero::Zero,
};

use crate::{CoinVisitor, IterNext};

type PassedThrough = bool;

pub(super) struct CurrencyFilter<'a, V, GIn, GFilter>
where
    GIn: Group,
    GFilter: Group,
{
    v: &'a mut V,
    _g_in: PhantomData<GIn>,
    filter: CurrencyDTO<GFilter>,
    filtered: Amount,
    pass_any: PassedThrough,
}
impl<'a, V, GIn, GFilter> CurrencyFilter<'a, V, GIn, GFilter>
where
    GIn: Group,
    GFilter: Group,
{
    pub fn new(v: &'a mut V, filter: CurrencyDTO<GFilter>) -> Self {
        Self {
            v,
            _g_in: PhantomData::<GIn>,
            filter,
            filtered: Amount::ZERO,
            pass_any: false,
        }
    }

    pub fn filtered(&self) -> Amount {
        self.filtered
    }

    #[cfg(debug_assertions)]
    pub fn passed_any(&self) -> PassedThrough {
        self.pass_any
    }
}
impl<V, GIn, GFilter> CoinVisitor for CurrencyFilter<'_, V, GIn, GFilter>
where
    V: CoinVisitor<GIn = GIn, Result = IterNext>,
    GIn: Group,
    GFilter: Group,
{
    type GIn = GIn;
    type Result = V::Result;
    type Error = V::Error;

    fn visit<GG>(&mut self, coin: &CoinDTO<GG>) -> Result<Self::Result, Self::Error>
    where
        GG: Group + MemberOf<Self::GIn>,
    {
        if coin.currency() == self.filter {
            self.filtered += coin.amount();
            Ok(IterNext::Continue)
        } else {
            self.pass_any = true;
            self.v.visit(coin)
        }
    }
}

#[cfg(test)]
mod test {
    use currency::{
        CurrencyDef, Group, MemberOf,
        test::{SubGroupTestC10, SuperGroup, SuperGroupTestC1, SuperGroupTestC2},
    };
    use finance::coin::{Amount, Coin, CoinDTO};

    use crate::{CoinVisitor, IterNext, impl_::swap_coins::TestVisitor};

    use super::CurrencyFilter;

    type FilterCurrency = SuperGroupTestC1;
    type AnotherCurrency = SuperGroupTestC2;
    type YetAnotherCurrency = SubGroupTestC10;
    const AMOUNT1: Amount = 24;
    const AMOUNT2: Amount = 28;

    #[test]
    fn filter() {
        let mut v = TestVisitor::<SuperGroup, IterNext>::new(IterNext::Continue, IterNext::Stop);
        let mut f = CurrencyFilter::new(&mut v, currency::dto::<FilterCurrency, SuperGroup>());

        assert_eq!(f.filtered(), 0);
        assert_passed_through::<false, _>(&f);

        assert_eq!(
            f.visit(&coin::<FilterCurrency>(AMOUNT1)).unwrap(),
            IterNext::Continue
        );
        assert_eq!(f.filtered(), AMOUNT1);
        assert_passed_through::<false, _>(&f);

        assert_eq!(
            f.visit(&coin::<FilterCurrency>(AMOUNT1)).unwrap(),
            IterNext::Continue
        );
        assert_eq!(f.filtered(), AMOUNT1 + AMOUNT1);
        assert_passed_through::<false, _>(&f);
    }

    #[test]
    fn pass_one() {
        let mut v = TestVisitor::<SuperGroup, IterNext>::new(IterNext::Stop, IterNext::Stop);
        let mut f = CurrencyFilter::new(&mut v, currency::dto::<FilterCurrency, SuperGroup>());

        assert_eq!(f.filtered(), 0);
        assert_passed_through::<false, _>(&f);

        assert_eq!(
            f.visit(&coin::<FilterCurrency>(AMOUNT1)).unwrap(),
            IterNext::Continue
        );
        assert_eq!(f.filtered(), AMOUNT1);
        assert_passed_through::<false, _>(&f);

        assert_eq!(
            f.visit(&coin::<AnotherCurrency>(AMOUNT1)).unwrap(),
            IterNext::Stop
        );
        assert_eq!(f.filtered(), AMOUNT1);
        assert_passed_through::<true, _>(&f);

        assert_eq!(
            f.visit(&Coin::<FilterCurrency>::new(AMOUNT2).into())
                .unwrap(),
            IterNext::Continue
        );
        assert_eq!(f.filtered(), AMOUNT1 + AMOUNT2);
        assert_passed_through::<true, _>(&f);

        assert!(v.first_visited(AMOUNT1));
        assert!(v.second_not_visited());
    }

    #[test]
    fn pass_two() {
        let mut v = TestVisitor::<SuperGroup, IterNext>::new(IterNext::Continue, IterNext::Stop);
        let mut f = CurrencyFilter::new(&mut v, currency::dto::<FilterCurrency, SuperGroup>());

        assert_eq!(f.filtered(), 0);
        assert_passed_through::<false, _>(&f);

        assert_eq!(
            f.visit(&coin::<AnotherCurrency>(AMOUNT1)).unwrap(),
            IterNext::Continue
        );
        assert_eq!(f.filtered(), 0);
        assert_passed_through::<true, _>(&f);

        assert_eq!(
            f.visit(&coin::<YetAnotherCurrency>(AMOUNT2)).unwrap(),
            IterNext::Stop
        );
        assert_eq!(f.filtered(), 0);
        assert_passed_through::<true, _>(&f);

        assert!(v.first_visited(AMOUNT1));
        assert!(v.second_visited(AMOUNT2));
    }

    fn assert_passed_through<const PASSED: bool, G>(
        _f: &CurrencyFilter<'_, TestVisitor<G, IterNext>, G, G>,
    ) where
        G: Group,
    {
        #[cfg(debug_assertions)]
        assert_eq!(_f.passed_any(), PASSED);
    }

    fn coin<C>(amount: Amount) -> CoinDTO<SuperGroup>
    where
        C: CurrencyDef,
        C::Group: MemberOf<SuperGroup>,
    {
        Coin::<C>::new(amount).into()
    }
}
