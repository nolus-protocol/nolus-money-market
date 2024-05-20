use currency::Currency;
use serde::Serialize;

use crate::msg::{BaseCurrencyQueryMsg, StableCurrencyQueryMsg};

pub trait RequestBuilder {
    fn currency() -> impl Serialize;

    fn price<C>() -> impl Serialize
    where
        C: Currency;
}

pub struct BasePriceRequest {}
impl RequestBuilder for BasePriceRequest {
    fn currency() -> impl Serialize {
        BaseCurrencyQueryMsg::BaseCurrency {}
    }

    fn price<C>() -> impl Serialize
    where
        C: Currency,
    {
        BaseCurrencyQueryMsg::BasePrice {
            currency: C::TICKER.to_string(),
        }
    }
}

pub struct StablePriceRequest {}
impl RequestBuilder for StablePriceRequest {
    fn currency() -> impl Serialize {
        StableCurrencyQueryMsg::StableCurrency {}
    }

    fn price<C>() -> impl Serialize
    where
        C: Currency,
    {
        StableCurrencyQueryMsg::StablePrice {
            currency: C::TICKER.to_string(),
        }
    }
}
