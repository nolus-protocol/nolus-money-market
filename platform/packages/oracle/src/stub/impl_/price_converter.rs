use std::marker::PhantomData;

use currency::{Currency, Group};
use finance::price::{self, dto::PriceDTO, Price};
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::{
    error::{Error, Result},
    Oracle, OracleRef,
};

use super::RequestBuilder;

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

#[cfg(feature = "unchecked-quote-currency")]
pub struct QuoteCUncheckedConverter();
#[cfg(feature = "unchecked-quote-currency")]
impl PriceConverter for QuoteCUncheckedConverter {
    fn try_convert<C, G, BaseC, BaseG>(price: PriceDTO<G, BaseG>) -> Result<Price<C, BaseC>>
    where
        C: Currency,
        G: Group,
        BaseC: Currency,
        BaseG: Group,
    {
        use finance::coin::Coin;

        price
            .base()
            .try_into()
            .map_err(Into::into)
            .map(|base| price::total_of(base).is(Into::<Coin<BaseC>>::into(price.quote().amount())))
    }
}

pub struct OracleStub<'a, QuoteC, QuoteG, PriceReq, PriceConverterT> {
    oracle_ref: OracleRef<QuoteC>,
    querier: QuerierWrapper<'a>,
    _quote_group: PhantomData<QuoteG>,
    _request: PhantomData<PriceReq>,
    _converter: PhantomData<PriceConverterT>,
}

impl<'a, QuoteC, QuoteG, PriceReq, PriceConverterT>
    OracleStub<'a, QuoteC, QuoteG, PriceReq, PriceConverterT>
where
    QuoteC: Currency,
    QuoteC::Group: Group<QuoteG>,
{
    pub fn new(oracle_ref: OracleRef<QuoteC>, querier: QuerierWrapper<'a>) -> Self {
        Self {
            oracle_ref,
            querier,
            _quote_group: PhantomData,
            _request: PhantomData,
            _converter: PhantomData,
        }
    }

    fn addr(&self) -> &Addr {
        &self.oracle_ref.addr
    }
}

impl<'a, QuoteC, QuoteG, PriceReqT, PriceConverterT> Oracle<QuoteC>
    for OracleStub<'a, QuoteC, QuoteG, PriceReqT, PriceConverterT>
where
    QuoteC: Currency,
    QuoteG: Group,
    PriceReqT: RequestBuilder,
    PriceConverterT: PriceConverter,
{
    fn price_of<C, G>(&self) -> Result<Price<C, QuoteC>>
    where
        C: Currency,
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

impl<'a, QuoteC, QuoteG, PriceReq, PriceConverterT> AsRef<OracleRef<QuoteC>>
    for OracleStub<'a, QuoteC, QuoteG, PriceReq, PriceConverterT>
{
    fn as_ref(&self) -> &OracleRef<QuoteC> {
        &self.oracle_ref
    }
}

impl<'a, QuoteC, QuoteG, PriceReq, PriceConverterT>
    From<OracleStub<'a, QuoteC, QuoteG, PriceReq, PriceConverterT>> for OracleRef<QuoteC>
{
    fn from(stub: OracleStub<'a, QuoteC, QuoteG, PriceReq, PriceConverterT>) -> Self {
        stub.oracle_ref
    }
}
