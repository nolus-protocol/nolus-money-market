use currency::{CurrencyDef, Group};
use finance::coin::{Coin, WithCoin};
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::{
    bank::{
        aggregate::Aggregate,
        send,
        view::{BalancesResult, BankAccountView},
    },
    batch::Batch,
    result::Result,
};

pub trait BankAccount
where
    Self: BankAccountView + Into<Batch>,
{
    fn send<C>(&mut self, amount: Coin<C>, to: Addr)
    where
        C: CurrencyDef;
}

pub struct BankStub<View>
where
    View: BankAccountView,
{
    view: View,
    batch: Batch,
}

impl<View> BankStub<View>
where
    View: BankAccountView,
{
    fn new(view: View) -> Self {
        Self {
            view,
            batch: Batch::default(),
        }
    }

    #[cfg(feature = "testing")]
    pub fn with_view(view: View) -> Self {
        Self::new(view)
    }
}

pub fn account<'a>(account: &'a Addr, querier: QuerierWrapper<'a>) -> impl BankAccount + use<'a> {
    BankStub::new(super::account_view(account, querier))
}

impl<View> BankAccountView for BankStub<View>
where
    View: BankAccountView,
{
    fn balance<C>(&self) -> Result<Coin<C>>
    where
        C: CurrencyDef,
    {
        self.view.balance()
    }

    fn balances<G, Cmd>(&self, cmd: Cmd) -> BalancesResult<G, Cmd>
    where
        G: Group,
        Cmd: WithCoin<G> + Clone,
        Cmd::Output: Aggregate,
    {
        self.view.balances::<G, Cmd>(cmd)
    }
}

impl<View> BankAccount for BankStub<View>
where
    Self: BankAccountView + Into<Batch>,
    View: BankAccountView,
{
    fn send<C>(&mut self, amount: Coin<C>, to: Addr)
    where
        C: CurrencyDef,
    {
        debug_assert!(!amount.is_zero());
        send::bank_send_impl(&mut self.batch, to, &[amount])
    }
}

impl<View> From<BankStub<View>> for Batch
where
    View: BankAccountView,
{
    fn from(stub: BankStub<View>) -> Self {
        stub.batch
    }
}
