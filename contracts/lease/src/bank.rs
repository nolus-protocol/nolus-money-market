use cosmwasm_std::{Addr, BankMsg, Coin, Env, QuerierWrapper, StdResult, SubMsg};

use crate::msg::Denom;

pub trait BankAccount {
    fn balance(&self, currency: &Denom) -> StdResult<Coin>;
    fn send(&self, to: &Addr, amount: Coin) -> StdResult<SubMsg>;
}

pub struct BankStub<'a> {
    addr: &'a Addr,
    querier: &'a QuerierWrapper<'a>,
}

impl<'a> BankStub<'a> {
    pub fn my_account(env: &'a Env, querier: &'a QuerierWrapper) -> Self {
        Self {
            addr: &env.contract.address,
            querier,
        }
    }
}
impl<'a> BankAccount for BankStub<'a> {
    fn balance(&self, currency: &Denom) -> StdResult<Coin> {
        self.querier.query_balance(self.addr, currency)
    }

    fn send(&self, to: &Addr, amount: Coin) -> StdResult<SubMsg> {
        Ok(SubMsg::new(BankMsg::Send {
            to_address: to.into(),
            amount: vec![amount],
        }))
    }
}
