use finance::{
    coin::CoinDTO,
    currency::{Group, Symbol},
};

use super::swap_task::{CoinVisitor, IterNext};

type PassedThrough = bool;

pub(super) struct CurrencyFilter<'a, V>(&'a mut V, Symbol<'a>, PassedThrough);
impl<'a, V> CurrencyFilter<'a, V> {
    pub fn new(v: &'a mut V, c: Symbol<'a>) -> Self {
        Self(v, c, false)
    }

    #[cfg(debug_assertions)]
    pub fn passed_through(&self) -> PassedThrough {
        self.2
    }
}
impl<'a, V> CoinVisitor for CurrencyFilter<'a, V>
where
    V: CoinVisitor<Result = IterNext>,
{
    type Result = V::Result;
    type Error = V::Error;

    fn visit<G>(&mut self, coin: &CoinDTO<G>) -> Result<Self::Result, Self::Error>
    where
        G: Group,
    {
        if coin.ticker() != self.1 {
            self.2 = true;
            self.0.visit(coin)
        } else {
            Ok(IterNext::Continue)
        }
    }
}

#[cfg(test)]
mod test {

    use finance::{
        coin::{Amount, Coin},
        currency::Currency,
        test::currency::{Dai, Nls, TestCurrencies, Usdc},
    };

    use crate::contract::state::opening::{
        swap_coins::TestVisitor,
        swap_task::{CoinVisitor, IterNext},
    };

    use super::CurrencyFilter;

    type FilterCurrency = Usdc;
    type AnotherCurrency = Nls;
    type YetAnotherCurrency = Dai;
    const AMOUNT1: Amount = 24;
    const AMOUNT2: Amount = 28;

    #[test]
    fn filter() {
        let mut v = TestVisitor::<IterNext>::new(IterNext::Continue, IterNext::Stop);
        let mut f = CurrencyFilter::new(&mut v, FilterCurrency::TICKER);

        assert_passed_through::<false>(&f);
        assert_eq!(
            f.visit::<TestCurrencies>(&Coin::<FilterCurrency>::new(AMOUNT1).into())
                .unwrap(),
            IterNext::Continue
        );
        assert_passed_through::<false>(&f);
        assert_eq!(
            f.visit::<TestCurrencies>(&Coin::<FilterCurrency>::new(AMOUNT1).into())
                .unwrap(),
            IterNext::Continue
        );
        assert_passed_through::<false>(&f);
    }

    #[test]
    fn pass_one() {
        let mut v = TestVisitor::<IterNext>::new(IterNext::Stop, IterNext::Stop);
        let mut f = CurrencyFilter::new(&mut v, FilterCurrency::TICKER);

        assert_passed_through::<false>(&f);
        assert_eq!(
            f.visit::<TestCurrencies>(&Coin::<FilterCurrency>::new(AMOUNT1).into())
                .unwrap(),
            IterNext::Continue
        );
        assert_passed_through::<false>(&f);

        assert_eq!(
            f.visit::<TestCurrencies>(&Coin::<AnotherCurrency>::new(AMOUNT1).into())
                .unwrap(),
            IterNext::Stop
        );
        assert_passed_through::<true>(&f);

        assert_eq!(
            f.visit::<TestCurrencies>(&Coin::<FilterCurrency>::new(AMOUNT2).into())
                .unwrap(),
            IterNext::Continue
        );
        assert_passed_through::<true>(&f);

        assert!(v.first_visited(AMOUNT1));
        assert!(v.second_not_visited());
    }

    #[test]
    fn pass_two() {
        let mut v = TestVisitor::<IterNext>::new(IterNext::Continue, IterNext::Stop);
        let mut f = CurrencyFilter::new(&mut v, FilterCurrency::TICKER);

        assert_passed_through::<false>(&f);
        assert_eq!(
            f.visit::<TestCurrencies>(&Coin::<AnotherCurrency>::new(AMOUNT1).into())
                .unwrap(),
            IterNext::Continue
        );
        assert_passed_through::<true>(&f);

        assert_eq!(
            f.visit::<TestCurrencies>(&Coin::<YetAnotherCurrency>::new(AMOUNT2).into())
                .unwrap(),
            IterNext::Stop
        );
        assert_passed_through::<true>(&f);

        assert!(v.first_visited(AMOUNT1));
        assert!(v.second_visited(AMOUNT2));
    }

    fn assert_passed_through<const PASSED: bool>(_f: &CurrencyFilter<'_, TestVisitor<IterNext>>) {
        #[cfg(debug_assertions)]
        assert_eq!(_f.passed_through(), PASSED);
    }
}
