use std::marker::PhantomData;

use currency::{Currency, Group, MemberOf};
use finance::price::{dto::PriceDTO, Price};
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::{
    error::{Error, Result},
    Oracle, OracleRef,
};

use super::RequestBuilder;

pub struct OracleStub<'a, QuoteC, QuoteG, PriceReq>
where
    QuoteC: Currency,
    QuoteG: Group,
{
    oracle_ref: OracleRef<QuoteC, QuoteG>,
    querier: QuerierWrapper<'a>,
    _request: PhantomData<PriceReq>,
}

impl<'a, QuoteC, QuoteG, PriceReq> OracleStub<'a, QuoteC, QuoteG, PriceReq>
where
    QuoteC: Currency,
    QuoteG: Group,
{
    pub fn new(oracle_ref: OracleRef<QuoteC, QuoteG>, querier: QuerierWrapper<'a>) -> Self {
        currency::validate_member::<QuoteC, QuoteG>()
            .expect("create OracleStub with an appropriate currency and a group");

        Self {
            oracle_ref,
            querier,
            _request: PhantomData,
        }
    }

    fn addr(&self) -> &Addr {
        &self.oracle_ref.addr
    }
}

impl<'a, QuoteC, QuoteG, PriceReqT> Oracle for OracleStub<'a, QuoteC, QuoteG, PriceReqT>
where
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
    PriceReqT: RequestBuilder,
{
    type QuoteC = QuoteC;
    type QuoteG = QuoteG;

    fn price_of<C, G>(&self) -> Result<Price<C, QuoteC>>
    where
        C: Currency + MemberOf<G>,
        G: Group,
    {
        if currency::equal::<C, QuoteC>() {
            return Ok(Price::identity());
        }

        let msg = PriceReqT::price::<C>();
        self.querier
            .query_wasm_smart(self.addr(), &msg)
            .map_err(|error| Error::FailedToFetchPrice {
                from: C::TICKER.into(),
                to: QuoteC::TICKER.into(),
                error,
            })
            .and_then(|price: PriceDTO<G, QuoteG>| price.try_into().map_err(Into::into))
    }
}

impl<'a, QuoteC, QuoteG, PriceReq> AsRef<OracleRef<QuoteC, QuoteG>>
    for OracleStub<'a, QuoteC, QuoteG, PriceReq>
where
    QuoteC: Currency,
    QuoteG: Group,
{
    fn as_ref(&self) -> &OracleRef<QuoteC, QuoteG> {
        &self.oracle_ref
    }
}

impl<'a, QuoteC, QuoteG, PriceReq> From<OracleStub<'a, QuoteC, QuoteG, PriceReq>>
    for OracleRef<QuoteC, QuoteG>
where
    QuoteC: Currency,
    QuoteG: Group,
{
    fn from(stub: OracleStub<'a, QuoteC, QuoteG, PriceReq>) -> Self {
        stub.oracle_ref
    }
}
