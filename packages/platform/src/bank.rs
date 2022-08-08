use cosmwasm_std::{Addr, BankMsg, Coin as CwCoin, Env, QuerierWrapper};

use finance::{coin::Coin, currency::Currency};

use crate::{
    batch::Batch,
    coin_legacy::{from_cosmwasm_impl, to_cosmwasm_impl},
    error::{Error, Result},
};

pub trait BankAccountView {
    fn balance<C>(&self) -> Result<Coin<C>>
    where
        C: Currency;
}

pub trait BankAccount: BankAccountView + Into<Batch> {
    fn send<C>(&mut self, amount: Coin<C>, to: &Addr)
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

pub struct BankView<'a> {
    addr: &'a Addr,
    querier: &'a QuerierWrapper<'a>,
}

impl<'a> BankView<'a> {
    pub fn my_account(env: &'a Env, querier: &'a QuerierWrapper) -> Self {
        Self {
            addr: &env.contract.address,
            querier,
        }
    }
}

impl<'a> BankAccountView for BankView<'a> {
    fn balance<C>(&self) -> Result<Coin<C>>
    where
        C: Currency,
    {
        let coin = self.querier.query_balance(self.addr, C::SYMBOL)?;
        from_cosmwasm_impl(coin)
    }
}

pub struct BankStub<'a> {
    view: BankView<'a>,
    batch: Batch,
}

impl<'a> BankStub<'a> {
    pub fn my_account(env: &'a Env, querier: &'a QuerierWrapper) -> Self {
        Self {
            view: BankView::my_account(env, querier),
            batch: Batch::default(),
        }
    }
}

impl<'a> BankAccountView for BankStub<'a> {
    fn balance<C>(&self) -> Result<Coin<C>>
    where
        C: Currency,
    {
        self.view.balance()
    }
}

impl<'a> BankAccount for BankStub<'a> {
    fn send<C>(&mut self, amount: Coin<C>, to: &Addr)
    where
        C: Currency,
    {
        self.batch.schedule_execute_no_reply(BankMsg::Send {
            to_address: to.into(),
            amount: vec![to_cosmwasm_impl(amount)],
        });
    }
}

impl<'a> From<BankStub<'a>> for Batch {
    fn from(stub: BankStub) -> Self {
        stub.batch
    }
}
