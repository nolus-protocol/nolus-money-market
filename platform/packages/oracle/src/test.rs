#![cfg(feature = "testing")]

use serde::Deserialize;

use currency::{test::SuperGroupTestC1, Currency, Group};
use finance::{
    coin::Amount,
    price::{self, Price},
};
use sdk::cosmwasm_std::StdError;

use crate::{
    error::{Error, Result},
    stub::Oracle,
    OracleRef,
};

pub struct DummyOracle(Option<Amount>);
impl DummyOracle {
    pub fn with_price(c_in_base: Amount) -> Self {
        Self(Some(c_in_base))
    }

    pub fn failing() -> Self {
        Self(None)
    }
}
impl<BaseC> Oracle<BaseC> for DummyOracle
where
    BaseC: Currency,
{
    fn price_of<C, G>(&self) -> Result<Price<C, BaseC>>
    where
        C: Currency,
        G: Group + for<'de> Deserialize<'de>,
    {
        self.0
            .map(|price| price::total_of(1.into()).is(price.into()))
            .ok_or_else(|| Error::FailedToFetchPrice {
                from: C::TICKER.into(),
                to: BaseC::TICKER.into(),
                error: StdError::GenericErr {
                    msg: "Test failing Oracle::price_of()".into(),
                },
            })
    }
}

impl From<DummyOracle> for OracleRef {
    fn from(_value: DummyOracle) -> Self {
        OracleRef::unchecked::<_, SuperGroupTestC1>("ADDR")
    }
}

impl AsRef<Self> for DummyOracle {
    fn as_ref(&self) -> &Self {
        self
    }
}
