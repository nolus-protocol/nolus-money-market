use std::marker::PhantomData;

use currency::{CurrencyDef, Group, MemberOf};
use finance::coin::{Coin, WithCoin};
use sdk::cosmwasm_std::{Addr, BankMsg, Coin as CwCoin, QuerierWrapper};

use crate::{
    bank::{account, view::BankAccountView},
    batch::Batch,
    coin_legacy,
    result::Result,
};

/// Send a single coin to a recepient
#[cfg(any(test, feature = "testing"))]
pub fn bank_send<C>(to: Addr, amount: Coin<C>) -> Batch
where
    C: CurrencyDef,
{
    let mut batch = Batch::default();
    bank_send_impl(&mut batch, to, &[amount]);
    batch
}

/// Send all coins to a recipient
pub fn bank_send_all<G>(from: &Addr, to: Addr, querier: QuerierWrapper<'_>) -> Result<Batch>
where
    G: Group,
{
    #[derive(Clone)]
    struct SendAny<G> {
        to: Addr,
        _g: PhantomData<G>,
    }

    impl<G> WithCoin<G> for SendAny<G>
    where
        G: Group,
    {
        type Outcome = Batch;

        fn on<C>(self, coin: Coin<C>) -> Self::Outcome
        where
            C: CurrencyDef,
            C::Group: MemberOf<G> + MemberOf<G::TopG>,
        {
            let mut sender = LazySenderStub::new(self.to);
            sender.send(coin);
            sender.into()
        }
    }

    let from_account = account::account(from, querier);
    from_account
        .balances(SendAny::<G> {
            to,
            _g: PhantomData,
        })
        .map(Option::unwrap_or_default)
}

pub trait FixedAddressSender
where
    Self: Into<Batch>,
{
    fn send<C>(&mut self, amount: Coin<C>)
    where
        C: CurrencyDef;
}

pub struct LazySenderStub {
    receiver: Addr,
    amounts: Vec<CwCoin>,
}

impl LazySenderStub {
    pub fn new(receiver: Addr) -> Self {
        Self {
            receiver,
            amounts: Vec::new(),
        }
    }
}

impl FixedAddressSender for LazySenderStub
where
    Self: Into<Batch>,
{
    fn send<C>(&mut self, amount: Coin<C>)
    where
        C: CurrencyDef,
    {
        if !amount.is_zero() {
            self.amounts.push(coin_legacy::to_cosmwasm_on_nolus(amount));
        }
    }
}

impl From<LazySenderStub> for Batch {
    fn from(stub: LazySenderStub) -> Self {
        let mut batch = Batch::default();

        if !stub.amounts.is_empty() {
            bank_send_cosmwasm(&mut batch, stub.receiver, stub.amounts);
        }

        batch
    }
}

pub(super) fn bank_send_impl<C>(batch: &mut Batch, to: Addr, amount: &[Coin<C>])
where
    C: CurrencyDef,
{
    bank_send_cosmwasm(
        batch,
        to,
        amount
            .iter()
            .map(|coin| coin_legacy::to_cosmwasm_on_nolus(coin.to_owned()))
            .collect(),
    )
}

fn bank_send_cosmwasm(batch: &mut Batch, to: Addr, amount: Vec<CwCoin>) {
    batch.schedule_execute_no_reply(BankMsg::Send {
        amount,
        to_address: to.into(),
    });
}
