use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use currency::Currency;
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::{
    api::{LpnQueryResponse, QueryMsg},
    error::{Error, Result},
};

use self::reserve::Impl;
pub use self::reserve::Reserve;

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
    Lpn: Currency,
{
    pub fn try_new(contract: Addr, querier: &QuerierWrapper<'_>) -> Result<Self> {
        querier
            .query_wasm_smart(contract.clone(), &QueryMsg::ReserveLpn())
            .map_err(Error::QueryReserveFailure)
            .and_then(|lpn: LpnQueryResponse| {
                currency::validate_ticker::<Lpn>(lpn).map_err(Error::UnexpectedLpn)
            })
            .map(|_| Self {
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
