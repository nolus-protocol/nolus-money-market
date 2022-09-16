use std::result::Result as StdResult;

use cosmwasm_std::{Addr, BankMsg, QuerierWrapper};
use serde::{Deserialize, Serialize};

use finance::{coin::Coin, currency::Currency};
use platform::{batch::Batch, coin_legacy::to_cosmwasm};

use crate::{
    msg::{ConfigResponse, QueryMsg},
    ContractError,
};

pub type Result<T> = StdResult<T, ContractError>;

pub struct ProfitBatch {
    pub profit_ref: ProfitRef,
    pub batch: Batch,
}

pub trait Profit
where
    Self: Into<ProfitBatch>,
{
    fn send<C>(&mut self, coins: Coin<C>) -> Result<()>
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
        cmd.exec(ProfitStub {
            profit_ref: self,
            batch: Batch::default(),
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

struct ProfitStub {
    profit_ref: ProfitRef,
    batch: Batch,
}

impl Profit for ProfitStub {
    fn send<C>(&mut self, coins: Coin<C>) -> Result<()>
    where
        C: Currency,
    {
        self.batch.schedule_execute_no_reply(BankMsg::Send {
            to_address: self.profit_ref.addr.to_string(),
            amount: vec![to_cosmwasm(coins)],
        });

        Ok(())
    }
}

impl From<ProfitStub> for ProfitBatch {
    fn from(stub: ProfitStub) -> Self {
        ProfitBatch {
            profit_ref: stub.profit_ref,
            batch: stub.batch,
        }
    }
}
