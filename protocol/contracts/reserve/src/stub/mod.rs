use std::marker::PhantomData;

use currency::{CurrencyDef, MemberOf};
use serde::{Deserialize, Serialize};

use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::api::{LpnCurrencies, LpnCurrencyDTO, QueryMsg};

use self::reserve::Impl;
pub use self::{error::Error, reserve::Reserve};

mod error;
mod reserve;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "testing"), derive(Eq, PartialEq))]
pub struct Ref<Lpn> {
    contract: Addr,
    #[serde(skip)]
    _lpns: PhantomData<Lpn>,
}

impl<Lpn> Ref<Lpn>
where
    Lpn: CurrencyDef,
    Lpn::Group: MemberOf<LpnCurrencies>,
{
    pub fn try_new(contract: Addr, querier: &QuerierWrapper<'_>) -> Result<Self, Error> {
        querier
            .query_wasm_smart(contract.clone(), &QueryMsg::ReserveLpn())
            .map_err(Error::QueryReserveFailure)
            .and_then(|lpn: LpnCurrencyDTO| {
                lpn.of_currency(&currency::dto::<Lpn, _>())
                    .map_err(Error::UnexpectedLpn)
            })
            .map(|()| Self {
                contract,
                _lpns: PhantomData,
            })
    }

    #[cfg(feature = "testing")]
    pub fn unchecked(contract: Addr) -> Self {
        Self {
            contract,
            _lpns: PhantomData,
        }
    }

    pub fn into_reserve(self) -> impl Reserve<Lpn> {
        Impl::new(self)
    }
}

impl<Lpn> From<Ref<Lpn>> for Addr {
    fn from(value: Ref<Lpn>) -> Self {
        value.contract
    }
}
