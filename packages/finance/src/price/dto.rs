use schemars::JsonSchema;
use serde::{
    de::DeserializeOwned,
    Deserialize, Serialize,
};

use crate::{
    coin::{Coin, CoinDTO},
    currency::{AnyVisitor, Currency, visit_any},
    error::Error,
    price::Price,
};

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
pub struct PriceDTO {
    amount: CoinDTO,
    amount_quote: CoinDTO,
}

impl<C, QuoteC> TryFrom<PriceDTO> for Price<C, QuoteC>
where
    C: Currency,
    QuoteC: Currency,
{
    type Error = Error;

    fn try_from(value: PriceDTO) -> Result<Self, Self::Error> {
        Ok(Price::new(
            value.amount.try_into()?,
            value.amount_quote.try_into()?,
        ))
    }
}

impl PriceDTO {
    pub fn new(base: CoinDTO, quote: CoinDTO) -> Self {
        Self {
            amount: base,
            amount_quote: quote,
        }
    }

    pub const fn base(&self) -> &CoinDTO {
        &self.amount
    }

    pub const fn quote(&self) -> &CoinDTO {
        &self.amount_quote
    }
}

impl<C, QuoteC> From<Price<C, QuoteC>> for PriceDTO
where
    C: Currency,
    QuoteC: Currency,
{
    fn from(price: Price<C, QuoteC>) -> Self {
        Self {
            amount: price.amount.into(),
            amount_quote: price.amount_quote.into(),
        }
    }
}

pub trait WithPrice {
    type Output;
    type Error;

    fn exec<C, QuoteC>(self, _: Price<C, QuoteC>) -> Result<Self::Output, Self::Error>
    where
        C: Currency,
        QuoteC: Currency;

    fn unknown(self) -> Result<Self::Output, Self::Error>;
}

pub fn visit_with_any_price<V>(price: PriceDTO, visitor: V) -> Result<V::Output, V::Error>
where
    V: WithPrice,
{
    visit_any(&price.amount.symbol().clone(), CVisitor {
        price_dto: price,
        visitor,
    })
}

struct CVisitor<InnerV1>
    where
    InnerV1: WithPrice
{
    price_dto: PriceDTO,
    visitor: InnerV1,
}

impl<InnerV1> AnyVisitor for CVisitor<InnerV1>
where
    InnerV1: WithPrice,
{
    type Output = InnerV1::Output;
    type Error = InnerV1::Error;

    fn on<C>(self) -> Result<Self::Output, Self::Error>
    where
        C: Currency + Serialize + DeserializeOwned,
    {
        visit_any(
            &self.price_dto.amount_quote.symbol().clone(),
            QuoteCVisitor {
                base: Coin::<C>::try_from(
                    self.price_dto.amount,
                ).expect("Got different currency in visitor!"),
                quote_dto: self.price_dto.amount_quote,
                visitor: self.visitor
            },
        )
    }

    fn on_unknown(self) -> Result<Self::Output, Self::Error> {
        self.visitor.unknown()
    }
}

struct QuoteCVisitor<InnerC, InnerV2>
    where
    InnerC: Currency + Serialize + DeserializeOwned,
    InnerV2: WithPrice
{
    base: Coin<InnerC>,
    quote_dto: CoinDTO,
    visitor: InnerV2,
}

impl<InnerC, InnerV2> AnyVisitor for QuoteCVisitor<InnerC, InnerV2>
where
    InnerC: Currency + Serialize + DeserializeOwned,
    InnerV2: WithPrice,
{
    type Output = InnerV2::Output;
    type Error = InnerV2::Error;

    fn on<QuoteC>(self) -> Result<Self::Output, Self::Error>
    where
        QuoteC: Currency + Serialize + DeserializeOwned,
    {
        self.visitor.exec(
            Price::new(
                self.base,
                Coin::<QuoteC>::try_from(
                    self.quote_dto,
                ).expect("Got different currency in visitor!"),
            ),
        )
    }

    fn on_unknown(self) -> Result<Self::Output, Self::Error> {
        self.visitor.unknown()
    }
}
