use cosmwasm_std::{Env, QuerierWrapper};
use finance::{
    coin::CoinDTO,
    currency::{Group, Symbol},
};
use oracle::stub::OracleRef;
use timealarms::stub::TimeAlarmsRef;

use crate::contract::dex::Account;

pub(super) type CoinsNb = u8;
pub(super) type OutChain = bool;
// pub(super) const LOCAL_OUT_CHAIN: OutChain = true;
pub(super) const REMOTE_OUT_CHAIN: OutChain = false;

/// Specification of a swap process
///
/// Supports up to `CoinsNb::MAX` coins.
pub(crate) trait SwapTask<OutG> {
    type Result;
    type Error;
    type Label: Into<String>;

    fn label(&self) -> Self::Label;
    fn dex_account(&self) -> &Account;
    fn oracle(&self) -> &OracleRef;
    fn time_alarm(&self, querier: &QuerierWrapper<'_>) -> Result<TimeAlarmsRef, Self::Error>;
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
        amount: CoinDTO<OutG>,
        querier: &QuerierWrapper<'_>,
        env: Env,
    ) -> Result<Self::Result, Self::Error>;
}

#[derive(PartialEq, Eq)]
#[cfg_attr(any(debug_assertions, test), derive(Debug))]
pub(crate) enum IterState {
    Complete,
    Incomplete,
}

#[derive(PartialEq, Eq)]
#[cfg_attr(test, derive(Clone, Debug))]
pub(crate) enum IterNext {
    Stop,
    Continue,
}

pub(crate) trait CoinVisitor {
    type Result;
    type Error;

    fn visit<G>(&mut self, coin: &CoinDTO<G>) -> Result<Self::Result, Self::Error>
    where
        G: Group;
}
