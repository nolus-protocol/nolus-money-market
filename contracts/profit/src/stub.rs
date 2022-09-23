use std::result::Result as StdResult;

use cosmwasm_std::{Addr, QuerierWrapper};
use serde::{Deserialize, Serialize};

use finance::{coin::Coin, currency::Currency};
use platform::bank::BankAccount;

use crate::{
    error::Result,
    msg::{ConfigResponse, QueryMsg},
};

pub trait Profit
where
    Self: Into<ProfitRef>,
{
    fn send<B, C>(&self, account: &mut B, amount: Coin<C>)
    where
        B: BankAccount,
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
        cmd.exec(ProfitStub { profit_ref: self })
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
}

impl Profit for ProfitStub {
    fn send<B, C>(&self, account: &mut B, amount: Coin<C>)
    where
        B: BankAccount,
        C: Currency,
    {
        account.send(amount, &self.profit_ref.addr);
    }
}

impl From<ProfitStub> for ProfitRef {
    fn from(stub: ProfitStub) -> Self {
        stub.profit_ref
    }
}
