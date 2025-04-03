use currency::{CurrencyDTO, Group, MemberOf};
use finance::coin::CoinDTO;
use oracle::stub::SwapPath;
use sdk::cosmwasm_std::{Env, QuerierWrapper};
use timealarms::stub::TimeAlarmsRef;

use crate::Account;

pub type CoinsNb = u8;

/// Specification of a swap process
///
/// Supports up to `CoinsNb::MAX` coins.
pub trait SwapTask {
    type InG: Group + MemberOf<Self::InOutG>;
    type OutG: Group + MemberOf<Self::InOutG>;
    type InOutG: Group;
    type Label: Into<String>;
    type StateResponse;
    type Result;

    fn label(&self) -> Self::Label;
    fn dex_account(&self) -> &Account;
    fn oracle(&self) -> &impl SwapPath<Self::InOutG>;
    fn time_alarm(&self) -> &TimeAlarmsRef;
    fn out_currency(&self) -> CurrencyDTO<Self::OutG>;

    /// Call back the worker with each coin this swap is about.
    /// The iteration is done over the coins always in the same order.
    /// It continues either until there are no more coins or the worker has responded
    /// with `IterNext::Stop` to the last call back.
    /// There should be at least one coin.
    fn on_coins<Visitor>(&self, visitor: &mut Visitor) -> Result<IterState, Visitor::Error>
    where
        Visitor: CoinVisitor<GIn = Self::InG, Result = IterNext>;

    /// The final transition of this DEX composite state machine
    ///
    /// The states involve TransferOut, SwapExactIn, TransferIn, etc. This transition originates from one of them,
    /// and should point to a next state, sibling to this one in the higher-level state machine.
    /// For example, the DEX [`Lease::BuyAsset`] state transition to [`Lease::Active`] on finish.
    ///
    fn finish(
        self,
        amount_out: CoinDTO<Self::OutG>,
        env: &Env,
        querier: QuerierWrapper<'_>,
    ) -> Self::Result;
}

#[derive(PartialEq, Eq)]
#[cfg_attr(any(debug_assertions, test, feature = "testing"), derive(Debug))]
pub enum IterState {
    Complete,
    Incomplete,
}

#[derive(PartialEq, Eq)]
#[cfg_attr(any(debug_assertions, test, feature = "testing"), derive(Clone, Debug))]
pub enum IterNext {
    Stop,
    Continue,
}

pub trait CoinVisitor {
    type GIn: Group;

    type Result;

    type Error;

    fn visit<G>(&mut self, coin: &CoinDTO<G>) -> Result<Self::Result, Self::Error>
    where
        G: Group + MemberOf<Self::GIn>;
}
