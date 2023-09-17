use serde::{Deserialize, Serialize};

use platform::bank::{FixedAddressSender, LazySenderStub};
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::{
    msg::{ConfigResponse, QueryMsg},
    result::ContractResult,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProfitRef {
    addr: Addr,
}

impl ProfitRef {
    pub fn new(addr: Addr, querier: &QuerierWrapper<'_>) -> ContractResult<Self> {
        querier
            .query_wasm_smart(addr.clone(), &QueryMsg::Config {})
            .map(|_: ConfigResponse| Self { addr })
            .map_err(Into::into)
    }

    pub fn into_stub(self) -> impl FixedAddressSender {
        LazySenderStub::new(self.addr)
    }
}

#[cfg(feature = "testing")]
impl ProfitRef {
    pub fn unchecked<A>(addr: A) -> Self
    where
        A: Into<String>,
    {
        Self {
            addr: Addr::unchecked(addr),
        }
    }
}
