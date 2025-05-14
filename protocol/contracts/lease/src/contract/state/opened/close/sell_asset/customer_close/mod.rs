use dex::{AcceptAnyNonZeroSwap, AnomalyHandler, AnomalyTreatment};
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
        state::{
            Response, event,
            opened::{close::Closable, payment::Repayable},
        },
    },
    error::ContractResult,
    finance::LpnCurrency,
    position::CloseStrategy,
};

use super::{Calculator as CloseCalculator, SellAsset, task::ClosePositionTask};

pub mod full;
pub mod partial;

type Calculator = AcceptAnyNonZeroSwap<LeaseAssetCurrencies, LpnCurrency>;
impl CloseCalculator for Calculator {}

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

impl<RepayableImpl> AnomalyHandler<SellAsset<RepayableImpl, Calculator>>
    for SellAsset<RepayableImpl, Calculator>
where
    RepayableImpl: Closable + Repayable,
{
    fn on_anomaly(self) -> AnomalyTreatment<SellAsset<RepayableImpl, Calculator>> {
        self.retry_on_anomaly()
    }
}
