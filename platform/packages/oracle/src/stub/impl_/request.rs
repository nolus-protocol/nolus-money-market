use currency::{Currency, CurrencyDTO, Group};
use serde::Serialize;

use crate::msg::{BaseCurrencyQueryMsg, StableCurrencyQueryMsg};

pub trait RequestBuilder {
    fn currency<G>() -> impl Serialize
    where
        G: Group;

    fn price<C>() -> impl Serialize
    where
        C: Currency;
}

pub struct BasePriceRequest {}
impl RequestBuilder for BasePriceRequest {
    fn currency<G>() -> impl Serialize
    where
        G: Group,
    {
        BaseCurrencyQueryMsg::<G>::BaseCurrency {}
    }

    fn price<C>() -> impl Serialize
    where
        C: Currency,
    {
        BaseCurrencyQueryMsg::BasePrice {
            currency: CurrencyDTO::<C::Group>::from_currency_type::<C>(),
        }
    }
}

pub struct StablePriceRequest {}
impl RequestBuilder for StablePriceRequest {
    fn currency<G>() -> impl Serialize
    where
        G: Group,
    {
        StableCurrencyQueryMsg::<G>::StableCurrency {}
    }

    fn price<C>() -> impl Serialize
    where
        C: Currency,
    {
        StableCurrencyQueryMsg::StablePrice {
            currency: CurrencyDTO::<C::Group>::from_currency_type::<C>(),
        }
    }
}
