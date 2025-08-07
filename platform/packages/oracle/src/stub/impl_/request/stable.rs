use serde::Serialize;

use currency::{CurrencyDef, Group};

use crate::msg::StableCurrencyQueryMsg;

use super::RequestBuilder;

pub struct StablePriceRequest {}
#[cfg(feature = "unchecked-stable-quote")]
impl RequestBuilder for StablePriceRequest {
    fn currency<G>() -> impl Serialize
    where
        G: Group,
    {
        StableCurrencyQueryMsg::<G>::StableCurrency {}
    }

    fn price<C>() -> impl Serialize
    where
        C: CurrencyDef,
    {
        StableCurrencyQueryMsg::StablePrice {
            currency: *C::dto(),
        }
    }
}
