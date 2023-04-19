use finance::{
    coin::CoinDTO,
    currency::{Group, Symbol},
};
use oracle::stub::OracleRef;
use sdk::cosmwasm_std::{Env, QuerierWrapper};
use timealarms::stub::TimeAlarmsRef;

use crate::account::Account;

pub type CoinsNb = u8;

/// Specification of a swap process
///
/// Supports up to `CoinsNb::MAX` coins.
pub trait SwapTask {
    type OutG: Group;
    type Label: Into<String>;
    type StateResponse;
    type Result;

    fn label(&self) -> Self::Label;
    fn dex_account(&self) -> &Account;
    fn oracle(&self) -> &OracleRef;
    fn time_alarm(&self) -> &TimeAlarmsRef;
    fn out_currency(&self) -> Symbol<'_>;

    /// Call back the worker with each coin this swap is about.
    /// The iteration is done over the coins always in the same order.
    /// It continues either until there are no more coins or the worker has responded
    /// with `IterNext::Stop` to the last call back.
    /// There should be at least one coin.
    fn on_coins<Visitor>(&self, visitor: &mut Visitor) -> Result<IterState, Visitor::Error>
    where
        Visitor: CoinVisitor<Result = IterNext>;

    fn finish(
        self,
        amount_out: CoinDTO<Self::OutG>,
        env: &Env,
        querier: &QuerierWrapper<'_>,
    ) -> Self::Result;
}

#[derive(PartialEq, Eq)]
#[cfg_attr(any(debug_assertions, test), derive(Debug))]
pub enum IterState {
    Complete,
    Incomplete,
}

#[derive(PartialEq, Eq)]
#[cfg_attr(test, derive(Clone, Debug))]
pub enum IterNext {
    Stop,
    Continue,
}

pub trait CoinVisitor {
    type Result;
    type Error;

    fn visit<G>(&mut self, coin: &CoinDTO<G>) -> Result<Self::Result, Self::Error>
    where
        G: Group;
}
