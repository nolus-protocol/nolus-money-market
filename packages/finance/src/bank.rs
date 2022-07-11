use cosmwasm_std::{Addr, BankMsg, Coin as CwCoin, Env, QuerierWrapper, SubMsg};

use crate::{
    coin::Coin,
    coin_legacy::{from_cosmwasm_impl, to_cosmwasm_impl},
    currency::Currency,
    error::{Result, Error},
};

pub trait BankAccount {
    fn balance<C>(&self) -> Result<Coin<C>>
    where
        C: Currency;

    fn send<C>(&self, amount: Coin<C>, to: &Addr) -> Result<SubMsg>
    where
        C: Currency;
}

pub fn received<C>(cw_amount: &[CwCoin]) -> Result<Coin<C>>
where
    C: Currency,
{
    match cw_amount.len() {
        0 => Err(Error::no_funds::<C>()),
        1 => {
            let cw_coin = &cw_amount[0];
            Ok(from_cosmwasm_impl(cw_coin.clone())?)
        }
        _ => Err(Error::unexpected_funds::<C>()),
    }
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
        from_cosmwasm_impl(coin)
    }

    fn send<C>(&self, amount: Coin<C>, to: &Addr) -> Result<SubMsg>
    where
        C: Currency,
    {
        Ok(SubMsg::new(BankMsg::Send {
            to_address: to.into(),
            amount: vec![to_cosmwasm_impl(amount)],
        }))
    }
}
