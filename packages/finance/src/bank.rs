use cosmwasm_std::{Addr, BankMsg, Env, QuerierWrapper, SubMsg};

use crate::{coin::Coin, error::Result, coin_legacy::{from_cosmwasm, to_cosmwasm}, currency::Currency};

pub trait BankAccount {
    fn balance<C>(&self) -> Result<Coin<C>>
    where
        C: Currency;

    fn send<C>(&self, amount: Coin<C>, to: &Addr) -> Result<SubMsg>
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
    fn balance<C>(&self) -> Result<Coin<C>>
    where
        C: Currency,
    {
        let coin = self.querier.query_balance(self.addr, C::SYMBOL)?;
        from_cosmwasm(coin)
    }

    fn send<C>(&self, amount: Coin<C>, to: &Addr) -> Result<SubMsg>
    where
        C: Currency,
    {
        Ok(SubMsg::new(BankMsg::Send {
            to_address: to.into(),
            amount: vec![to_cosmwasm(amount)],
        }))
    }
}
