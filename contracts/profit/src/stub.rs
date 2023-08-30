use serde::{Deserialize, Serialize};

use currency::Currency;
use finance::coin::Coin;
use platform::{
    bank::{FixedAddressSender, LazySenderStub},
    batch::Batch,
};
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::{
    msg::{ConfigResponse, QueryMsg},
    result::ContractResult,
};

pub trait Profit
where
    Self: Into<Batch>,
{
    fn send<C>(&mut self, amount: Coin<C>)
    where
        C: Currency;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProfitRef {
    addr: Addr,
}

impl ProfitRef {
    pub fn new(addr: Addr, querier: &QuerierWrapper<'_>) -> ContractResult<Self> {
        let _: ConfigResponse = querier.query_wasm_smart(addr.clone(), &QueryMsg::Config {})?;

        Ok(Self { addr })
    }

    pub fn as_stub(&self) -> impl FixedAddressSender {
        LazySenderStub::new(self.addr.clone())
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
