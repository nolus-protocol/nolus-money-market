use serde::Serialize;

use currency::{CurrencyDef, Group};

use crate::msg::BaseCurrencyQueryMsg;

use super::RequestBuilder;

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
        C: CurrencyDef,
    {
        BaseCurrencyQueryMsg::BasePrice {
            currency: *C::dto(),
        }
    }
}
