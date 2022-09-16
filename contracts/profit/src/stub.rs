use std::result::Result as StdResult;

use cosmwasm_std::{Addr, BankMsg, Coin as CwCoin, QuerierWrapper, Uint128};
use serde::{Deserialize, Serialize};

use finance::{
    coin::{Amount, Coin},
    currency::Currency,
};
use platform::{batch::Batch, coin_legacy::to_cosmwasm};

use crate::{
    error::Result,
    msg::{ConfigResponse, QueryMsg},
    ContractError,
};

pub struct ProfitBatch {
    pub profit_ref: ProfitRef,
    pub batch: Batch,
}

pub trait Profit
where
    Self: Into<ProfitBatch>,
{
    fn send<C>(&mut self, amount: Coin<C>) -> Result<()>
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
            coins: Vec::new(),
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
    coins: Vec<CwCoin>,
}

impl Profit for ProfitStub {
    fn send<C>(&mut self, amount: Coin<C>) -> Result<()>
    where
        C: Currency,
    {
        if amount.is_zero() {
            return Ok(());
        }

        if let Some(coin) = self
            .coins
            .iter_mut()
            .find(|amount| amount.denom == C::SYMBOL)
        {
            coin.amount += Uint128::new(Amount::from(amount));
        } else {
            self.coins.push(to_cosmwasm(amount));
        }

        Ok(())
    }
}

impl From<ProfitStub> for ProfitBatch {
    fn from(stub: ProfitStub) -> Self {
        let mut batch = Batch::default();

        if !stub.coins.is_empty() {
            batch.schedule_execute_no_reply(BankMsg::Send {
                to_address: stub.profit_ref.addr.to_string(),
                amount: stub.coins,
            });
        }

        ProfitBatch {
            profit_ref: stub.profit_ref,
            batch,
        }
    }
}
