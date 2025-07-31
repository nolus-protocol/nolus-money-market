use std::cell::OnceCell;

use currency::{CurrencyDef, Group};
use finance::coin::{Coin, WithCoin};
use sdk::cosmwasm_std::Addr;

use crate::{
    bank::{
        account::BankAccount,
        aggregate::Aggregate,
        view::{BalancesResult, BankAccountView},
    },
    batch::Batch,
    result::Result,
};

pub fn not_taking_balance() -> impl BankAccountView {
    struct PanickingView();
    impl BankAccountView for PanickingView {
        fn balance<C>(&self) -> Result<Coin<C>>
        where
            C: CurrencyDef,
        {
            unimplemented!(
                "Unexpected call of BankAccountView::balance with C = {}",
                C::ticker()
            )
        }

        fn balances<G, Cmd>(&self, _cmd: Cmd) -> BalancesResult<G, Cmd>
        where
            G: Group,
            Cmd: WithCoin<G> + Clone,
            Cmd::Output: Aggregate,
        {
            unimplemented!()
        }
    }
    PanickingView()
}

pub fn take_balance_once<C, OthersView>(
    balance: Coin<C>,
    others_view: OthersView,
) -> impl BankAccountView
where
    C: 'static,
    OthersView: BankAccountView,
{
    TakeBalanceOnce::new(balance, others_view)
}

/// Provide given balance only once
struct TakeBalanceOnce<C, View> {
    balance: Coin<C>,
    query_done: OnceCell<bool>,
    others_view: View,
}

impl<C, View> TakeBalanceOnce<C, View> {
    fn new(balance: Coin<C>, others_view: View) -> Self {
        Self {
            balance,
            query_done: OnceCell::new(),
            others_view,
        }
    }
}

impl<C, View> BankAccountView for TakeBalanceOnce<C, View>
where
    C: 'static,
    View: BankAccountView,
{
    fn balance<CC>(&self) -> Result<Coin<CC>>
    where
        CC: CurrencyDef,
    {
        if currency::equal::<C, CC>() {
            match self.query_done.get() {
                Some(_done) => panic!("Unexpected query of balance {}", CC::ticker()),
                None => {
                    let set_res = self.query_done.set(true);
                    debug_assert_eq!(Ok(()), set_res);
                    Ok(self.balance.coerce_into())
                }
            }
        } else {
            self.others_view.balance()
        }
    }

    fn balances<G, Cmd>(&self, _cmd: Cmd) -> BalancesResult<G, Cmd>
    where
        G: Group,
        Cmd: WithCoin<G> + Clone,
        Cmd::Output: Aggregate,
    {
        unimplemented!()
    }
}

//TODO refactor to like to the MockBank implementation
#[derive(Clone)]
pub struct MockBankView<OneC, OtherC> {
    balance: Coin<OneC>,
    balance_other: Coin<OtherC>,
}

impl<OneC, OtherC> MockBankView<OneC, OtherC> {
    pub fn new(balance: Coin<OneC>, balance_other: Coin<OtherC>) -> Self {
        Self {
            balance,
            balance_other,
        }
    }
    pub fn only_balance(balance: Coin<OneC>) -> Self {
        Self {
            balance,
            balance_other: Coin::default(),
        }
    }
}

impl<OneC, OtherC> BankAccountView for MockBankView<OneC, OtherC>
where
    OneC: CurrencyDef,
    OtherC: CurrencyDef,
{
    fn balance<C>(&self) -> Result<Coin<C>>
    where
        C: CurrencyDef,
    {
        if currency::equal::<C, OneC>() {
            Ok(self.balance.coerce_into())
        } else if currency::equal::<C, OtherC>() {
            Ok(self.balance_other.coerce_into())
        } else {
            unreachable!(
                "Expected {} or {}, found {}",
                currency::to_string(OneC::dto()),
                currency::to_string(OtherC::dto()),
                currency::to_string(C::dto())
            );
        }
    }

    fn balances<G, Cmd>(&self, _: Cmd) -> BalancesResult<G, Cmd>
    where
        G: Group,
        Cmd: WithCoin<G>,
        Cmd::Output: Aggregate,
    {
        unimplemented!()
    }
}

pub fn no_transfers<BankView>(view: BankView) -> impl BankAccount
where
    BankView: BankAccountView,
{
    PanickingBank::with_view(view)
}

pub fn one_transfer<TransferC, BankView>(
    amount: Coin<TransferC>,
    to: Addr,
    view: BankView,
) -> impl BankAccount
where
    TransferC: 'static,
    BankView: BankAccountView,
{
    MockBank::single_transfer(amount, to, view)
}

pub fn two_transfers<TransferC1, TransferC2, BankView>(
    amount1: Coin<TransferC1>,
    to1: Addr,
    amount2: Coin<TransferC2>,
    to2: Addr,
    view: BankView,
) -> impl BankAccount
where
    TransferC1: 'static,
    TransferC2: 'static,
    BankView: BankAccountView,
{
    MockBank::prev_transfer(amount1, to1, MockBank::single_transfer(amount2, to2, view))
}

/// A bank account testing implementation employing the chain-of-responsability design pattern
struct MockBank<TransferC, Bank> {
    expected_transfer: Coin<TransferC>,
    expected_recepient: Addr,
    transfer_met: bool,
    next: Bank,
}

impl<TransferC, BankView> MockBank<TransferC, PanickingBank<BankView>> {
    fn single_transfer(amount: Coin<TransferC>, to: Addr, view: BankView) -> Self {
        Self {
            expected_transfer: amount,
            expected_recepient: to,
            transfer_met: false,
            next: PanickingBank::with_view(view),
        }
    }
}

impl<TransferC, Bank> MockBank<TransferC, Bank> {
    fn prev_transfer(amount: Coin<TransferC>, to: Addr, next: Bank) -> Self {
        Self {
            expected_transfer: amount,
            expected_recepient: to,
            transfer_met: false,
            next,
        }
    }
}

impl<TransferC, Bank> BankAccountView for MockBank<TransferC, Bank>
where
    TransferC: 'static,
    Bank: BankAccount,
{
    fn balance<C>(&self) -> Result<Coin<C>>
    where
        C: CurrencyDef,
    {
        self.next.balance()
    }

    fn balances<G, Cmd>(&self, _cmd: Cmd) -> BalancesResult<G, Cmd>
    where
        G: Group,
        Cmd: WithCoin<G> + Clone,
        Cmd::Output: Aggregate,
    {
        unimplemented!()
    }
}

impl<TransferC, Bank> BankAccount for MockBank<TransferC, Bank>
where
    TransferC: 'static,
    Bank: BankAccount,
{
    fn send<C>(&mut self, transfer: Coin<C>, to: Addr)
    where
        C: 'static + CurrencyDef,
    {
        if self.transfer_met {
            self.next.send(transfer, to);
        } else {
            assert!(currency::equal::<TransferC, C>());
            assert_eq!(self.expected_transfer, transfer.coerce_into());
            assert_eq!(self.expected_recepient, to);
            self.transfer_met = true;
        }
    }
}

impl<TransferC, Bank> From<MockBank<TransferC, Bank>> for Batch {
    fn from(_value: MockBank<TransferC, Bank>) -> Self {
        Batch::default() //no messages since the mock has fulfuilled its job at this stage
    }
}

/// A null-pattern implementation of `Bank`
struct PanickingBank<BankView> {
    view: BankView,
}
impl<BankView> PanickingBank<BankView> {
    fn with_view(view: BankView) -> Self {
        Self { view }
    }
}
impl<BankView> BankAccountView for PanickingBank<BankView>
where
    BankView: BankAccountView,
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
        self.view.balances(cmd)
    }
}
impl<BankView> BankAccount for PanickingBank<BankView>
where
    BankView: BankAccountView,
{
    fn send<C>(&mut self, amount: Coin<C>, to: Addr)
    where
        C: CurrencyDef,
    {
        unimplemented!("Unexpected transfer to '{to}' for {amount}!")
    }
}
impl<BankView> From<PanickingBank<BankView>> for Batch {
    fn from(_value: PanickingBank<BankView>) -> Self {
        Self::default()
    }
}
