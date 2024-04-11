use std::marker::PhantomData;

use serde::Deserialize;

use currency::{Currency, Group};
use finance::price::{dto::PriceDTO, Price};
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::api::QueryMsg;

use super::{
    error::{Error, Result},
    Oracle, OracleRef,
};

trait PriceConverter {
    fn try_convert<C, G, BaseC, BaseG>(dto: PriceDTO<G, BaseG>) -> Result<Price<C, BaseC>>
    where
        C: Currency,
        G: Group,
        BaseC: Currency,
        BaseG: Group;
}
pub struct CheckedConverter();
impl PriceConverter for CheckedConverter {
    fn try_convert<C, G, BaseC, BaseG>(price: PriceDTO<G, BaseG>) -> Result<Price<C, BaseC>>
    where
        C: Currency,
        G: Group,
        BaseC: Currency,
        BaseG: Group,
    {
        price.try_into().map_err(Into::into)
    }
}

#[cfg(feature = "unchecked-base-currency")]
pub struct BaseCUncheckedConverter();
#[cfg(feature = "unchecked-base-currency")]
impl PriceConverter for BaseCUncheckedConverter {
    fn try_convert<C, G, BaseC, BaseG>(price: PriceDTO<G, BaseG>) -> Result<Price<C, BaseC>>
    where
        C: Currency,
        G: Group,
        BaseC: Currency,
        BaseG: Group,
    {
        use finance::{coin::Coin, price};

        price
            .base()
            .try_into()
            .map_err(Into::into)
            .map(|base| price::total_of(base).is(Into::<Coin<BaseC>>::into(price.quote().amount())))
    }
}

pub struct OracleStub<'a, OracleBase, OracleBaseG, PriceConverterT> {
    oracle_ref: OracleRef,
    querier: QuerierWrapper<'a>,
    _quote_currency: PhantomData<OracleBase>,
    _quote_group: PhantomData<OracleBaseG>,
    _converter: PhantomData<PriceConverterT>,
}

impl<'a, OracleBase, OracleBaseG, PriceConverterT>
    OracleStub<'a, OracleBase, OracleBaseG, PriceConverterT>
where
    OracleBase: Currency,
    OracleBaseG: Group,
{
    pub fn new(oracle_ref: OracleRef, querier: QuerierWrapper<'a>) -> Self {
        currency::validate_member::<OracleBase, OracleBaseG>()
            .expect("create OracleStub with an appropriate currency and a group");

        Self {
            oracle_ref,
            querier,
            _quote_currency: PhantomData,
            _quote_group: PhantomData,
            _converter: PhantomData,
        }
    }

    fn addr(&self) -> &Addr {
        &self.oracle_ref.addr
    }
}

impl<'a, OracleBase, OracleBaseG, PriceConverterT> Oracle<OracleBase>
    for OracleStub<'a, OracleBase, OracleBaseG, PriceConverterT>
where
    OracleBase: Currency,
    OracleBaseG: Group + for<'de> Deserialize<'de>,
    PriceConverterT: PriceConverter,
{
    fn price_of<C, G>(&self) -> Result<Price<C, OracleBase>>
    where
        C: ?Sized + Currency,
        G: Group,
    {
        if currency::equal::<C, OracleBase>() {
            return Ok(Price::identity());
        }

        let msg = QueryMsg::BasePrice {
            currency: C::TICKER.to_string(),
        };
        self.querier
            .query_wasm_smart(self.addr(), &msg)
            .map_err(|error| Error::FailedToFetchPrice {
                from: C::TICKER.into(),
                to: OracleBase::TICKER.into(),
                error,
            })
            .and_then(PriceConverterT::try_convert::<C, G, OracleBase, OracleBaseG>)
    }
}

impl<'a, OracleBase, OracleBaseG, PriceConverterT>
    AsRef<OracleStub<'a, OracleBase, OracleBaseG, PriceConverterT>>
    for OracleStub<'a, OracleBase, OracleBaseG, PriceConverterT>
{
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<'a, OracleBase, OracleBaseG, PriceConverterT>
    From<OracleStub<'a, OracleBase, OracleBaseG, PriceConverterT>> for OracleRef
{
    fn from(stub: OracleStub<'a, OracleBase, OracleBaseG, PriceConverterT>) -> Self {
        stub.oracle_ref
    }
}
