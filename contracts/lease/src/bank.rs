use cosmwasm_std::{Addr, BankMsg, Env, QuerierWrapper, SubMsg};
use finance::{
    coin::{Coin, Currency},
    coin_legacy::{to_cosmwasm, from_cosmwasm},
};

use crate::error::ContractResult;

pub trait BankAccount {
    fn balance<C>(&self) -> ContractResult<Coin<C>>
    where
        C: Currency;

    fn send<C>(&self, amount: Coin<C>, to: &Addr) -> ContractResult<SubMsg>
    where
        C: Currency;
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
    fn balance<C>(&self) -> ContractResult<Coin<C>>
    where
        C: Currency,
    {
        let coin = self.querier.query_balance(self.addr, C::SYMBOL)?;
        from_cosmwasm(coin).map_err(|e|e.into())
    }

    fn send<C>(&self, amount: Coin<C>, to: &Addr) -> ContractResult<SubMsg>
    where
        C: Currency,
    {
        Ok(SubMsg::new(BankMsg::Send {
            to_address: to.into(),
            amount: vec![to_cosmwasm(amount)],
        }))
    }
}
