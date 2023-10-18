use currency::{lpn::Lpns, Currency};
use serde::{Deserialize, Serialize};

use error::Result;
use finance::coin::{Amount, Coin};
use sdk::{
    cosmwasm_std::{Addr, QuerierWrapper},
    schemars::{self, JsonSchema},
};
use stub::LppStub;

pub use crate::nlpn::NLpn;
pub use crate::usd::Usd;

pub mod error;
pub mod msg;
mod nlpn;
mod stub;
mod usd;

pub trait Lpp {
    fn balance(&self) -> Result<LppBalanceResponse>;
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct LppBalanceResponse {
    pub balance: Coin<Usd>,
    pub total_principal_due: Coin<Usd>,
    pub total_interest_due: Coin<Usd>,
    pub balance_nlpn: Coin<NLpn>,
}

pub fn new_stub<'a>(lpp: Addr, querier: &'a QuerierWrapper<'a>) -> impl Lpp + 'a {
    LppStub::new(lpp, querier)
}

#[cfg(feature = "impl")]
pub fn into_usd<Lpn>(amount: Coin<Lpn>) -> Coin<Usd>
where
    Lpn: Currency,
{
    debug_assert_eq!(currency::validate_member::<Lpn, Lpns>(), Ok(()));

    Into::<Amount>::into(amount).into()
}
