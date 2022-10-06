use std::result::Result as StdResult;

use cosmwasm_std::{Addr, QuerierWrapper};
use serde::{Deserialize, Serialize};

use finance::{coin::Coin, currency::Currency};
use platform::{
    bank::{FixedAddressSender, LazySenderStub},
    batch::Batch,
};

use crate::{
    error::Result,
    msg::{ConfigResponse, QueryMsg},
};

pub struct ProfitBatch {
    pub profit_ref: ProfitRef,
    pub batch: Batch,
}

pub trait Profit
where
    Self: Into<ProfitBatch>,
{
    fn send<C>(&mut self, amount: Coin<C>)
    where
        C: Currency;
}

pub trait WithProfit {
    type Output;
    type Error;

    fn exec<P>(self, profit: P) -> StdResult<Self::Output, Self::Error>
    where
        P: Profit;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProfitRef {
    addr: Addr,
}

impl From<ProfitRef> for Addr {
    fn from(profit_ref: ProfitRef) -> Self {
        profit_ref.addr
    }
}

impl ProfitRef {
    pub fn try_from(addr: Addr, querier: &QuerierWrapper) -> Result<Self> {
        let _: ConfigResponse = querier.query_wasm_smart(addr.clone(), &QueryMsg::Config {})?;

        Ok(Self { addr })
    }

    pub fn execute<Cmd>(self, cmd: Cmd) -> StdResult<Cmd::Output, Cmd::Error>
    where
        Cmd: WithProfit,
    {
        let profit_address = self.addr.clone();

        cmd.exec(ProfitStub {
            profit_ref: self,
            sender: LazySenderStub::new(profit_address),
        })
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

struct ProfitStub<Sender> {
    profit_ref: ProfitRef,
    sender: Sender,
}

impl<Sender> Profit for ProfitStub<Sender>
where
    Sender: FixedAddressSender,
{
    fn send<C>(&mut self, amount: Coin<C>)
    where
        C: Currency,
    {
        self.sender.send(amount);
    }
}

impl<Sender> From<ProfitStub<Sender>> for ProfitBatch
where
    Sender: FixedAddressSender,
{
    fn from(stub: ProfitStub<Sender>) -> Self {
        ProfitBatch {
            profit_ref: stub.profit_ref,
            batch: stub.sender.into(),
        }
    }
}
