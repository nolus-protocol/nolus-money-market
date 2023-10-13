use currency::{Group, SymbolSlice};
use finance::{
    coin::{Amount, CoinDTO},
    zero::Zero,
};

use crate::{CoinVisitor, IterNext};

type PassedThrough = bool;

pub(super) struct CurrencyFilter<'a, V> {
    v: &'a mut V,
    filter: &'a SymbolSlice,
    filtered: Amount,
    pass_any: PassedThrough,
}
impl<'a, V> CurrencyFilter<'a, V> {
    pub fn new(v: &'a mut V, filter: &'a SymbolSlice) -> Self {
        Self {
            v,
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
        if coin.ticker() == self.filter {
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
        test::{Dai, Nls, TestCurrencies, TestExtraCurrencies, Usdc},
        Currency,
    };
    use finance::coin::{Amount, Coin};

    use crate::{
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

        assert_eq!(f.filtered(), 0);
        assert_passed_through::<false>(&f);

        assert_eq!(
            f.visit::<TestCurrencies>(&Coin::<FilterCurrency>::new(AMOUNT1).into())
                .unwrap(),
            IterNext::Continue
        );
        assert_eq!(f.filtered(), AMOUNT1);
        assert_passed_through::<false>(&f);

        assert_eq!(
            f.visit::<TestCurrencies>(&Coin::<FilterCurrency>::new(AMOUNT1).into())
                .unwrap(),
            IterNext::Continue
        );
        assert_eq!(f.filtered(), AMOUNT1 + AMOUNT1);
        assert_passed_through::<false>(&f);
    }

    #[test]
    fn pass_one() {
        let mut v = TestVisitor::<IterNext>::new(IterNext::Stop, IterNext::Stop);
        let mut f = CurrencyFilter::new(&mut v, FilterCurrency::TICKER);

        assert_eq!(f.filtered(), 0);
        assert_passed_through::<false>(&f);

        assert_eq!(
            f.visit::<TestCurrencies>(&Coin::<FilterCurrency>::new(AMOUNT1).into())
                .unwrap(),
            IterNext::Continue
        );
        assert_eq!(f.filtered(), AMOUNT1);
        assert_passed_through::<false>(&f);

        assert_eq!(
            f.visit::<TestCurrencies>(&Coin::<AnotherCurrency>::new(AMOUNT1).into())
                .unwrap(),
            IterNext::Stop
        );
        assert_eq!(f.filtered(), AMOUNT1);
        assert_passed_through::<true>(&f);

        assert_eq!(
            f.visit::<TestCurrencies>(&Coin::<FilterCurrency>::new(AMOUNT2).into())
                .unwrap(),
            IterNext::Continue
        );
        assert_eq!(f.filtered(), AMOUNT1 + AMOUNT2);
        assert_passed_through::<true>(&f);

        assert!(v.first_visited(AMOUNT1));
        assert!(v.second_not_visited());
    }

    #[test]
    fn pass_two() {
        let mut v = TestVisitor::<IterNext>::new(IterNext::Continue, IterNext::Stop);
        let mut f = CurrencyFilter::new(&mut v, FilterCurrency::TICKER);

        assert_eq!(f.filtered(), 0);
        assert_passed_through::<false>(&f);

        assert_eq!(
            f.visit::<TestCurrencies>(&Coin::<AnotherCurrency>::new(AMOUNT1).into())
                .unwrap(),
            IterNext::Continue
        );
        assert_eq!(f.filtered(), 0);
        assert_passed_through::<true>(&f);

        assert_eq!(
            f.visit::<TestExtraCurrencies>(&Coin::<YetAnotherCurrency>::new(AMOUNT2).into())
                .unwrap(),
            IterNext::Stop
        );
        assert_eq!(f.filtered(), 0);
        assert_passed_through::<true>(&f);

        assert!(v.first_visited(AMOUNT1));
        assert!(v.second_visited(AMOUNT2));
    }

    fn assert_passed_through<const PASSED: bool>(_f: &CurrencyFilter<'_, TestVisitor<IterNext>>) {
        #[cfg(debug_assertions)]
        assert_eq!(_f.passed_any(), PASSED);
    }
}
