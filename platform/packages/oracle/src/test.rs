use currency::{CurrencyDef, Group, MemberOf, platform::Stable};
use finance::{
    coin::{Amount, Coin},
    price::{self, Price},
};
use sdk::cosmwasm_std::{Addr, StdError};

use crate::{
    StablePriceSource,
    error::{self, Result},
    stub::Oracle,
};

pub struct DummyOracle {
    source: StablePriceSource,
    price: Option<Amount>,
}

impl DummyOracle {
    pub fn with_price(c_in_base: Amount) -> Self {
        Self {
            source: Self::dummy_source(),
            price: Some(c_in_base),
        }
    }

    pub fn failing() -> Self {
        Self {
            source: Self::dummy_source(),
            price: None,
        }
    }

    fn dummy_source() -> StablePriceSource {
        StablePriceSource::new(Addr::unchecked("ADDR"), String::from("USDC_TEST"))
    }
}

impl<G> Oracle<G> for DummyOracle
where
    G: Group,
{
    type QuoteC = Stable;
    type QuoteG = <Self::QuoteC as CurrencyDef>::Group;

    fn price_of<C>(&self) -> Result<Price<C, Self::QuoteC>>
    where
        C: CurrencyDef,
        C::Group: MemberOf<G>,
    {
        self.price
            .map(|price| price::total_of(Coin::new(1)).is(Coin::new(price)))
            .ok_or_else(|| {
                error::failed_to_fetch_price(
                    C::dto(),
                    Self::QuoteC::dto(),
                    StdError::generic_err("Test failing Oracle::price_of()"),
                )
            })
    }
}

impl AsRef<StablePriceSource> for DummyOracle {
    fn as_ref(&self) -> &StablePriceSource {
        &self.source
    }
}

#[cfg(test)]
mod test_convert {
    use currency::{platform::Stable, test::SuperGroupTestC1};
    use finance::coin::{Amount, Coin};

    use super::DummyOracle;
    use crate::convert;

    #[test]
    fn from_quote() {
        assert_from_quote(3, 12, 4);
        assert_from_quote(1, 4, 4);
        assert_from_quote(2, 14, 7);
        assert_from_quote(2, Amount::MAX, Amount::MAX / 2);
        assert_from_quote(Amount::MAX / 5, 4, 20 / Amount::MAX);
        assert_from_quote(Amount::MAX, 5, 0);
    }

    #[test]
    fn to_quote() {
        assert_to_quote(4, 3, 12);
        assert_to_quote(1, 6, 6);
        assert_to_quote(10, 4, 40);
        assert_to_quote(7, 1, 7);
        assert_to_quote(Amount::MAX / 10, 5, Amount::MAX / 2 - 2);
    }

    #[test]
    fn to_quote_overflow() {
        let oracle = DummyOracle::with_price(Amount::MAX / 4);
        assert!(convert::to_quote(&oracle, Coin::<SuperGroupTestC1>::new(8)).is_err());
    }

    fn assert_from_quote(oracle_price: Amount, in_amount: Amount, expected_out: Amount) {
        let oracle = DummyOracle::with_price(oracle_price);

        let out_amount = convert::from_quote(&oracle, Coin::new(in_amount)).unwrap();

        assert_eq!(Coin::<SuperGroupTestC1>::new(expected_out), out_amount);
    }

    fn assert_to_quote(oracle_price: Amount, in_amount: Amount, expected_out: Amount) {
        let oracle = DummyOracle::with_price(oracle_price);
        let out_amount =
            convert::to_quote(&oracle, Coin::<SuperGroupTestC1>::new(in_amount)).unwrap();
        assert_eq!(Coin::<Stable>::new(expected_out), out_amount);
    }
}
