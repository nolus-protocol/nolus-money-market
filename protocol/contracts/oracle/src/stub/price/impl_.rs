use std::marker::PhantomData;

use currency::{Currency, Group};
use finance::price::{dto::PriceDTO, Price};
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::api::price::QueryMsg;

use super::{
    error::{Error, Result},
    Oracle, OracleRef,
};

pub struct OracleStub<'a, OracleBase, OracleBaseG> {
    oracle_ref: OracleRef<OracleBase>,
    querier: QuerierWrapper<'a>,
    _quote_group: PhantomData<OracleBaseG>,
}

impl<'a, OracleBase, OracleBaseG> OracleStub<'a, OracleBase, OracleBaseG>
where
    OracleBase: Currency,
    OracleBaseG: Group,
{
    pub fn new(oracle_ref: OracleRef<OracleBase>, querier: QuerierWrapper<'a>) -> Self {
        currency::validate_member::<OracleBase, OracleBaseG>()
            .expect("create OracleStub with an appropriate currency and a group");

        Self {
            oracle_ref,
            querier,
            _quote_group: PhantomData,
        }
    }

    fn addr(&self) -> &Addr {
        &self.oracle_ref.addr
    }
}

impl<'a, OracleBase, OracleBaseG> Oracle<OracleBase> for OracleStub<'a, OracleBase, OracleBaseG>
where
    OracleBase: Currency,
    OracleBaseG: Group,
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
            .and_then(|price: PriceDTO<G, OracleBaseG>| price.try_into().map_err(Into::into))
    }
}

impl<'a, OracleBase, OracleBaseG> From<OracleStub<'a, OracleBase, OracleBaseG>>
    for OracleRef<OracleBase>
{
    fn from(stub: OracleStub<'a, OracleBase, OracleBaseG>) -> Self {
        stub.oracle_ref
    }
}
