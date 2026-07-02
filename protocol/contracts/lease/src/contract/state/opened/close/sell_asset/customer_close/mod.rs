use dex::AcceptAnyNonZeroSwap;
use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::{Env, QuerierWrapper};

use crate::{
    api::{
        LeaseAssetCurrencies,
        position::{FullClose, PositionClose},
    },
    contract::{
        Lease,
        cmd::ValidateClosePosition,
        state::{Response, event},
    },
    error::ContractResult,
    finance::LpnCurrency,
    position::CloseStrategy,
};

use super::{Calculator as CloseCalculator, task::ClosePositionTask};

pub mod full;
pub mod partial;

type Calculator = AcceptAnyNonZeroSwap<LeaseAssetCurrencies, LpnCurrency>;
impl CloseCalculator for Calculator {}

pub(crate) type PartialDrainState = super::DrainState<partial::RepayableImpl>;
pub(crate) type FullDrainState = super::DrainState<full::RepayableImpl>;

pub fn start(
    close: PositionClose,
    lease: Lease,
    env: &Env,
    querier: QuerierWrapper<'_>,
) -> ContractResult<Response> {
    match close {
        PositionClose::PartialClose(spec) => lease
            .lease
            .clone()
            .execute(ValidateClosePosition::new(&spec), querier)
            .and_then(|()| {
                spec.start(
                    lease,
                    MessageResponse::default(),
                    Calculator::default(),
                    env,
                    querier,
                )
            }),
        PositionClose::FullClose(spec) => spec.start(
            lease,
            MessageResponse::default(),
            Calculator::default(),
            env,
            querier,
        ),
    }
}

pub fn auto_start(
    strategy: CloseStrategy,
    lease: Lease,
    env: &Env,
    querier: QuerierWrapper<'_>,
) -> ContractResult<Response> {
    let events = event::emit_auto_close(strategy, env, &lease.lease.addr);
    FullClose {}.start(lease, events.into(), Calculator::default(), env, querier)
}

#[cfg(all(feature = "internal.test.contract", test))]
mod tests {
    use dex::SlippageCalculator;

    use crate::api::LeaseAssetCurrencies;

    use super::Calculator;

    /// Truth table (#660): the customer-close legs run `AcceptAnyNonZeroSwap`,
    /// which keeps the `REQUOTES_ON_TIMEOUT` default of `false` — a
    /// customer-close timeout re-emits the pinned floor verbatim, unlike the
    /// liquidation legs sharing the same `SellAsset` spec.
    /// COMPILE-RED: blocked on `SlippageCalculator::REQUOTES_ON_TIMEOUT`.
    #[test]
    fn customer_close_calculator_does_not_requote_on_timeout() {
        assert!(!<Calculator as SlippageCalculator<LeaseAssetCurrencies>>::REQUOTES_ON_TIMEOUT);
    }
}
