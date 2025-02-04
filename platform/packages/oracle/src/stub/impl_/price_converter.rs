use std::marker::PhantomData;

use currency::{Currency, CurrencyDef, Group, MemberOf};
use finance::price::{base::BasePrice, Price};
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::{
    error::{self, Result},
    Oracle, OracleRef,
};

use super::RequestBuilder;

pub struct OracleStub<'a, G, QuoteC, QuoteG, PriceReq>
where
    G: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    oracle_ref: OracleRef<QuoteC, QuoteG>,
    querier: QuerierWrapper<'a>,
    _group: PhantomData<G>,
    _request: PhantomData<PriceReq>,
}

impl<'a, G, QuoteC, QuoteG, PriceReq> OracleStub<'a, G, QuoteC, QuoteG, PriceReq>
where
    G: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    pub fn new(oracle_ref: OracleRef<QuoteC, QuoteG>, querier: QuerierWrapper<'a>) -> Self {
        Self {
            oracle_ref,
            querier,
            _group: PhantomData,
            _request: PhantomData,
        }
    }

    fn addr(&self) -> &Addr {
        &self.oracle_ref.addr
    }
}

impl<CurrencyG, TopG, QuoteC, QuoteG, PriceReqT> Oracle<CurrencyG>
    for OracleStub<'_, TopG, QuoteC, QuoteG, PriceReqT>
where
    CurrencyG: Group + MemberOf<TopG>,
    TopG: Group,
    QuoteC: CurrencyDef,
    QuoteC::Group: MemberOf<QuoteG> + MemberOf<CurrencyG::TopG>,
    QuoteG: Group,
    PriceReqT: RequestBuilder,
{
    type QuoteC = QuoteC;
    type QuoteG = QuoteG;

    fn price_of<C>(&self) -> Result<Price<C, QuoteC>>
    where
        C: CurrencyDef,
        C::Group: MemberOf<CurrencyG>,
    {
        if currency::equal::<C, QuoteC>() {
            return Ok(Price::identity());
        }

        let msg = PriceReqT::price::<C>();
        self.querier
            .query_wasm_smart(self.addr(), &msg)
            .map_err(|error| error::failed_to_fetch_price(C::dto(), QuoteC::dto(), error))
            .and_then(|price: BasePrice<CurrencyG, QuoteC, QuoteG>| {
                price.try_into().map_err(Into::into)
            })
    }
}

impl<TopG, QuoteC, QuoteG, PriceReq> AsRef<OracleRef<QuoteC, QuoteG>>
    for OracleStub<'_, TopG, QuoteC, QuoteG, PriceReq>
where
    TopG: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    fn as_ref(&self) -> &OracleRef<QuoteC, QuoteG> {
        &self.oracle_ref
    }
}

impl<'a, TopG, QuoteC, QuoteG, PriceReq> From<OracleStub<'a, TopG, QuoteC, QuoteG, PriceReq>>
    for OracleRef<QuoteC, QuoteG>
where
    TopG: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    fn from(stub: OracleStub<'a, TopG, QuoteC, QuoteG, PriceReq>) -> Self {
        stub.oracle_ref
    }
}
