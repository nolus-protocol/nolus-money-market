use currency::{
    CurrencyDef, Group, MemberOf, SymbolOwned, SymbolRef,
    platform::{PlatformGroup, Stable},
};
use finance::price::{Price, base::ExternalPrice};
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::{
    Oracle,
    error::{self, Error, Result},
    msg::StableCurrencyQueryMsg,
};

/// Describe a price source contract
pub struct PriceSource {
    addr: Addr,
    quote_ticker: SymbolOwned,
}

impl PriceSource {
    pub fn new(addr: Addr, quote_ticker: SymbolOwned) -> Self {
        Self { addr, quote_ticker }
    }

    pub fn addr(&self) -> &Addr {
        &self.addr
    }

    pub fn quote_ticker(&self) -> SymbolRef<'_> {
        &self.quote_ticker
    }
}

/// An implementation of [`Oracle<G>`] for prices with [`Stable`] as a quote currency
///
/// Handling of different and unknown stable protocol currencies is done through 'coercion'.
/// Refer to [`ExternalPrice`] for more details.
pub struct PriceStub<'a> {
    source: PriceSource,
    querier: QuerierWrapper<'a>,
}

impl<'a> PriceStub<'a> {
    pub fn try_new(oracle_addr: Addr, querier: QuerierWrapper<'a>) -> Result<Self> {
        querier
            .query_wasm_smart(
                oracle_addr.clone(),
                &StableCurrencyQueryMsg::<PlatformGroup>::StableCurrency {},
            )
            .map_err(Error::StubConfigQuery)
            .map(|stable_ticker: SymbolOwned| Self {
                source: PriceSource::new(oracle_addr, stable_ticker),
                querier,
            })
    }
}

impl<G> Oracle<G> for PriceStub<'_>
where
    G: Group,
{
    type QuoteC = Stable;
    type QuoteG = <Stable as CurrencyDef>::Group;

    fn price_of<C>(&self) -> Result<Price<C, Self::QuoteC>>
    where
        C: CurrencyDef,
        C::Group: MemberOf<G>,
    {
        if currency::equal::<C, Self::QuoteC>() {
            return Ok(Price::identity());
        }

        let req = StableCurrencyQueryMsg::StablePrice {
            currency: *C::dto(),
        };
        self.querier
            .query_wasm_smart(self.source.addr.clone(), &req)
            .map_err(|error| error::failed_to_fetch_price(C::dto(), Self::QuoteC::dto(), error))
            .and_then(|price: ExternalPrice<G, Self::QuoteC>| {
                price
                    .try_coerce(&self.source.quote_ticker)
                    .map_err(Into::into)
            })
            .and_then(|price| price.try_into().map_err(Into::into))
    }
}

impl AsRef<PriceSource> for PriceStub<'_> {
    fn as_ref(&self) -> &PriceSource {
        &self.source
    }
}
