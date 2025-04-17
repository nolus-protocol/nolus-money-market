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

    /// Provide the coins, at least one, this swap is about.
    /// The iteration is done always in the same order.
    //
    // TODO define the Item type as an associative : AsRef<CoinDTO<Self::InG>> to allow iterating over values and references.
    // This would avoid clone-ing of values kept in the task. At the same time, we cannot iterate over '&' due to
    // having temporary instances in some of the tasks.
    fn coins(&self) -> impl IntoIterator<Item = CoinDTO<Self::InG>>;

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
