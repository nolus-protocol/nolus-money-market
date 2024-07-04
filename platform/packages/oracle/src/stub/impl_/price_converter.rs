use std::marker::PhantomData;

use currency::{group::MemberOf, Currency, CurrencyDTO, Group};
#[cfg(feature = "unchecked-quote-currency")]
use finance::price::base::BasePrice;
use finance::price::{dto::PriceDTO, Price};
use sdk::cosmwasm_std::{Addr, QuerierWrapper};
use serde::Deserialize;

use crate::{
    error::{self, Result},
    Oracle, OracleRef,
};

use super::RequestBuilder;

trait PriceConverter {
    type G: Group;
    type QuoteC: Currency + MemberOf<Self::QuoteG>;
    type QuoteG: Group;
    type From;

    fn convert<C>(from: Self::From) -> Price<C, Self::QuoteC>
    where
        C: Currency + MemberOf<Self::G>;
}
pub struct CheckedConverter<G, QuoteC, QuoteG>(
    PhantomData<G>,
    PhantomData<QuoteC>,
    PhantomData<QuoteG>,
);
impl<G, QuoteC, QuoteG> PriceConverter for CheckedConverter<G, QuoteC, QuoteG>
where
    G: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    type G = G;
    type QuoteC = QuoteC;
    type QuoteG = QuoteG;
    type From = PriceDTO<G, QuoteG>;

    fn convert<C>(from: Self::From) -> Price<C, QuoteC>
    where
        C: Currency + MemberOf<G>,
    {
        from.into()
    }
}

#[cfg(feature = "unchecked-quote-currency")]
pub struct QuoteCUncheckedConverter<G, QuoteC, QuoteG>(
    PhantomData<G>,
    PhantomData<QuoteC>,
    PhantomData<QuoteG>,
);
#[cfg(feature = "unchecked-quote-currency")]
impl<G, QuoteC, QuoteG> PriceConverter for QuoteCUncheckedConverter<G, QuoteC, QuoteG>
where
    G: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    type G = G;

    type QuoteC = QuoteC;

    type QuoteG = QuoteG;

    type From = BasePrice<G, QuoteC>;

    fn convert<C>(price: Self::From) -> Price<C, QuoteC>
    where
        C: Currency + MemberOf<Self::G>,
    {
        (&price).into()
    }
}

pub struct OracleStub<'a, G, QuoteC, QuoteG, PriceReq, PriceConverterT> {
    oracle_ref: OracleRef<QuoteC, QuoteG>,
    querier: QuerierWrapper<'a>,
    _group: PhantomData<G>,
    _request: PhantomData<PriceReq>,
    _converter: PhantomData<PriceConverterT>,
}

impl<'a, G, QuoteC, QuoteG, PriceReq, PriceConverterT>
    OracleStub<'a, G, QuoteC, QuoteG, PriceReq, PriceConverterT>
where
    QuoteC: Currency + MemberOf<QuoteG>,
{
    pub fn new(oracle_ref: OracleRef<QuoteC, QuoteG>, querier: QuerierWrapper<'a>) -> Self {
        Self {
            oracle_ref,
            querier,
            _group: PhantomData,
            _request: PhantomData,
            _converter: PhantomData,
        }
    }

    fn addr(&self) -> &Addr {
        &self.oracle_ref.addr
    }
}

impl<'a, G, QuoteC, QuoteG, PriceReqT, PriceConverterT> Oracle
    for OracleStub<'a, G, QuoteC, QuoteG, PriceReqT, PriceConverterT>
where
    G: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
    PriceReqT: RequestBuilder,
    PriceConverterT: PriceConverter<G = G, QuoteG = QuoteG, QuoteC = QuoteC>,
    PriceConverterT::From: for<'de> Deserialize<'de>,
{
    type G = G;
    type QuoteC = QuoteC;
    type QuoteG = QuoteG;

    fn price_of<C>(&self) -> Result<Price<C, Self::QuoteC>, Self::G>
    where
        C: Currency + MemberOf<Self::G>,
    {
        if currency::equal::<C, QuoteC>() {
            return Ok(Price::identity());
        }

        let msg = PriceReqT::price::<C>();
        self.querier
            .query_wasm_smart(self.addr(), &msg)
            .map_err(|error| {
                error::failed_to_fetch_price(
                    CurrencyDTO::<G>::from_currency_type::<C>(),
                    CurrencyDTO::<QuoteG>::from_currency_type::<QuoteC>(),
                    error,
                )
            })
            .map(PriceConverterT::convert::<C>)
    }
}

impl<'a, G, QuoteC, QuoteG, PriceReq, PriceConverterT> AsRef<OracleRef<QuoteC, QuoteG>>
    for OracleStub<'a, G, QuoteC, QuoteG, PriceReq, PriceConverterT>
{
    fn as_ref(&self) -> &OracleRef<QuoteC, QuoteG> {
        &self.oracle_ref
    }
}

impl<'a, G, QuoteC, QuoteG, PriceReq, PriceConverterT>
    From<OracleStub<'a, G, QuoteC, QuoteG, PriceReq, PriceConverterT>>
    for OracleRef<QuoteC, QuoteG>
{
    fn from(stub: OracleStub<'a, G, QuoteC, QuoteG, PriceReq, PriceConverterT>) -> Self {
        stub.oracle_ref
    }
}
