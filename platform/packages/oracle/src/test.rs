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
