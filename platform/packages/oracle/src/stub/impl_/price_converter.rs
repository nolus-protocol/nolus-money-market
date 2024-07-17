use std::marker::PhantomData;

use currency::{Currency, Group, MemberOf};
use finance::price::{dto::PriceDTO, Price};
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::{
    error::{Error, Result},
    Oracle, OracleRef,
};

use super::RequestBuilder;

// TODO [ref: new_unchecked_stable_quote_stub]
trait PriceConverter {
    fn try_convert<C, G, BaseC, BaseG>(dto: PriceDTO<G, BaseG>) -> Result<Price<C, BaseC>>
    where
        C: Currency + MemberOf<G>,
        G: Group,
        BaseC: Currency + MemberOf<BaseG>,
        BaseG: Group;
}
pub struct CheckedConverter();
impl PriceConverter for CheckedConverter {
    fn try_convert<C, G, BaseC, BaseG>(price: PriceDTO<G, BaseG>) -> Result<Price<C, BaseC>>
    where
        C: Currency + MemberOf<G>,
        G: Group,
        BaseC: Currency + MemberOf<BaseG>,
        BaseG: Group,
    {
        price.try_into().map_err(Into::into)
    }
}

#[cfg(feature = "unchecked-stable-quote")]
pub struct QuoteCUncheckedConverter();
#[cfg(feature = "unchecked-stable-quote")]
impl PriceConverter for QuoteCUncheckedConverter {
    fn try_convert<C, G, BaseC, BaseG>(price: PriceDTO<G, BaseG>) -> Result<Price<C, BaseC>>
    where
        C: Currency + MemberOf<G>,
        G: Group,
        BaseC: Currency + MemberOf<BaseG>,
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

pub struct OracleStub<'a, QuoteC, QuoteG, PriceReq, PriceConverterT>
where
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    oracle_ref: OracleRef<QuoteC, QuoteG>,
    querier: QuerierWrapper<'a>,
    _request: PhantomData<PriceReq>,
    _converter: PhantomData<PriceConverterT>,
}

impl<'a, QuoteC, QuoteG, PriceReq, PriceConverterT>
    OracleStub<'a, QuoteC, QuoteG, PriceReq, PriceConverterT>
where
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    pub fn new(oracle_ref: OracleRef<QuoteC, QuoteG>, querier: QuerierWrapper<'a>) -> Self {
        Self {
            oracle_ref,
            querier,
            _request: PhantomData,
            _converter: PhantomData,
        }
    }

    fn addr(&self) -> &Addr {
        &self.oracle_ref.addr
    }
}

impl<'a, QuoteC, QuoteG, PriceReqT, PriceConverterT> Oracle
    for OracleStub<'a, QuoteC, QuoteG, PriceReqT, PriceConverterT>
where
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
    PriceReqT: RequestBuilder,
    PriceConverterT: PriceConverter,
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
            .and_then(PriceConverterT::try_convert::<C, G, QuoteC, QuoteG>)
    }
}

impl<'a, QuoteC, QuoteG, PriceReq, PriceConverterT> AsRef<OracleRef<QuoteC, QuoteG>>
    for OracleStub<'a, QuoteC, QuoteG, PriceReq, PriceConverterT>
where
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    fn as_ref(&self) -> &OracleRef<QuoteC, QuoteG> {
        &self.oracle_ref
    }
}

impl<'a, QuoteC, QuoteG, PriceReq, PriceConverterT>
    From<OracleStub<'a, QuoteC, QuoteG, PriceReq, PriceConverterT>> for OracleRef<QuoteC, QuoteG>
where
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    fn from(stub: OracleStub<'a, QuoteC, QuoteG, PriceReq, PriceConverterT>) -> Self {
        stub.oracle_ref
    }
}
