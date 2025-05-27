use dex::{AcceptUpToMaxSlippage, AnomalyHandler, AnomalyTreatment};
use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::{Env, QuerierWrapper};

use crate::{
    api::query::opened::Cause as ApiCause,
    contract::{
        Lease,
        cmd::LiquidationDTO,
        state::{
            Response,
            opened::{
                close::{Closable, SlippageAnomaly},
                event,
                payment::Repayable,
            },
        },
    },
    error::ContractResult,
    position::Cause,
};

use super::{SellAsset, migrate_v0_8_7::CompoundCalculator, task::ClosePositionTask};

pub mod full;
pub mod partial;

// TODO switch to the following implementation on the next release
// type Calculator = MaxSlippage<LeaseAssetCurrencies, LpnCurrency, LpnCurrencies>;
type Calculator = CompoundCalculator;
impl super::Calculator for Calculator {}

pub fn start(
    lease: Lease,
    liquidation: LiquidationDTO,
    curr_request_response: MessageResponse,
    env: &Env,
    querier: QuerierWrapper<'_>,
) -> ContractResult<Response> {
    lease
        .leases
        .max_slippage(querier)
        .map(|max_slippage| {
            AcceptUpToMaxSlippage::with(max_slippage.liquidation, lease.lease.oracle.clone()).into()
        })
        .and_then(|slippage_calc| match liquidation {
            LiquidationDTO::Partial(spec) => {
                spec.start(lease, curr_request_response, slippage_calc, env, querier)
            }
            LiquidationDTO::Full(spec) => {
                spec.start(lease, curr_request_response, slippage_calc, env, querier)
            }
        })
}

impl From<Cause> for ApiCause {
    fn from(value: Cause) -> Self {
        match value {
            Cause::Liability {
                ltv: _,
                healthy_ltv: _,
            } => ApiCause::Liability,
            Cause::Overdue() => ApiCause::Overdue,
        }
    }
}

impl<RepayableImpl> AnomalyHandler<SellAsset<RepayableImpl, Calculator>>
    for SellAsset<RepayableImpl, Calculator>
where
    RepayableImpl: Closable + Repayable,
{
    fn on_anomaly(self) -> AnomalyTreatment<SellAsset<RepayableImpl, Calculator>> {
        let emitter =
            event::emit_slippage_anomaly(&self.lease.lease, self.slippage_calc.threshold());
        let next_state = SlippageAnomaly::new(self.lease);
        AnomalyTreatment::Exit(Ok(Response::from(emitter, next_state)))
    }
}
