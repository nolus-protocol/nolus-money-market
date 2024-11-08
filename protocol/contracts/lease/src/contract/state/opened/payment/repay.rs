use serde::{Deserialize, Serialize};

use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::{Env, QuerierWrapper};

use crate::{
    api::{
        query::opened::{OngoingTrx, PositionCloseTrx},
        LeaseCoin,
    },
    contract::{
        cmd::{CloseStatusDTO, Repay as RepayCmd, RepayEmitter, RepayFn, RepayResult},
        state::{
            opened::{
                active, alarm,
                close::{liquidation, Closable},
            },
            paid, Response,
        },
        Lease,
    },
    error::ContractResult,
    event::Type,
    finance::LpnCoinDTO,
};

use super::Repayable;

pub(crate) trait RepayAlgo {
    type RepayFn: RepayFn;
    type PaymentEmitter<'this, 'env>: RepayEmitter
    where
        Self: 'this;

    fn repay_fn(&self) -> Self::RepayFn;
    fn emitter_fn<'this, 'env>(&'this self, env: &'env Env) -> Self::PaymentEmitter<'this, 'env>;
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Repay<RepayAlgoT>(RepayAlgoT)
where
    RepayAlgoT: RepayAlgo;
impl<RepayAlgoT> From<RepayAlgoT> for Repay<RepayAlgoT>
where
    RepayAlgoT: RepayAlgo,
{
    fn from(value: RepayAlgoT) -> Self {
        Self(value)
    }
}

impl<RepayAlgoT> Closable for Repay<RepayAlgoT>
where
    RepayAlgoT: RepayAlgo + Closable,
{
    fn amount<'a>(&'a self, lease: &'a Lease) -> &'a LeaseCoin {
        self.0.amount(lease)
    }

    fn transaction(&self, lease: &Lease, in_progress: PositionCloseTrx) -> OngoingTrx {
        self.0.transaction(lease, in_progress)
    }

    fn event_type(&self) -> Type {
        self.0.event_type()
    }
}

impl<RepayAlgoT> Repayable for Repay<RepayAlgoT>
where
    RepayAlgoT: RepayAlgo,
{
    fn try_repay(
        &self,
        lease: Lease,
        amount: LpnCoinDTO,
        env: &Env,
        querier: QuerierWrapper<'_>,
    ) -> ContractResult<Response> {
        let profit = lease.lease.loan.profit().clone();
        let price_alarms = lease.lease.oracle.clone();
        let time_alarms = lease.lease.time_alarms.clone();
        let reserve = lease.lease.reserve.clone();
        let (
            lease,
            RepayResult {
                response,
                loan_paid,
                close_status,
            },
        ) = lease.update(
            RepayCmd::new(
                self.0.repay_fn(),
                amount,
                &env.block.time,
                self.0.emitter_fn(env),
                profit,
                (time_alarms, price_alarms),
                reserve,
            ),
            querier,
        )?;

        match close_status {
            CloseStatusDTO::Paid => Ok(finish_repay(loan_paid, response, lease)),
            CloseStatusDTO::None {
                current_liability,
                alarms,
            } => {
                let response =
                    alarm::build_resp(&lease, current_liability, alarms).merge_with(response);
                Ok(finish_repay(loan_paid, response, lease))
            }
            CloseStatusDTO::NeedLiquidation(liquidation) => {
                liquidation::start(lease, liquidation, response, env, querier)
            }
            CloseStatusDTO::CloseAsked(_strategy) => {
                todo!("TODO reset the Stop-Loss or Take-Profit trigger fired after payment (incl. liquidation or manual close)
                 and check again the lease close status")
            }
        }
    }
}

fn finish_repay(loan_paid: bool, repay_response: MessageResponse, lease: Lease) -> Response {
    if loan_paid {
        Response::from(repay_response, paid::Active::new(lease))
    } else {
        Response::from(repay_response, active::Active::new(lease))
    }
}
