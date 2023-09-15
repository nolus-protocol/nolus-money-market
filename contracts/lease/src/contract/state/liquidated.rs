use platform::message::Response as MessageResponse;
use serde::{Deserialize, Serialize};

use sdk::cosmwasm_std::{Deps, Env, MessageInfo, QuerierWrapper, Timestamp};

use crate::{
    api::{LpnCoin, StateResponse},
    contract::{
        cmd::{FullLiquidation, LiquidationDTO},
        Lease,
    },
    error::ContractResult,
};

use super::{event::LiquidationEmitter, Handler, Response};

#[derive(Serialize, Deserialize, Default)]
pub struct Liquidated {}

impl Liquidated {
    pub(super) fn enter_state(
        &self,
        lease: Lease,
        liquidation_descr: (LiquidationDTO, LpnCoin),
        now: Timestamp,
        env: &Env,
        querier: &QuerierWrapper<'_>,
    ) -> ContractResult<MessageResponse> {
        let liquidation = liquidation_descr.0;
        let liquidation_lpn = liquidation_descr.1;
        let liquidation_amount = liquidation.amount(&lease.lease).clone();
        let customer = lease.lease.customer.clone();
        let profit = lease.lease.loan.profit().clone();

        lease.finalizer.notify(customer).and_then(|finalizer_msgs| {
            lease
                .execute(
                    FullLiquidation::new(
                        liquidation_lpn,
                        now,
                        LiquidationEmitter::new(liquidation, liquidation_amount, env),
                        profit,
                    ),
                    querier,
                )
                .map(|liquidation_response| liquidation_response.merge_with(finalizer_msgs))
            //make sure the finalizer messages go out last
        })
    }
}

impl Handler for Liquidated {
    fn state(
        self,
        _now: Timestamp,
        _querier: &QuerierWrapper<'_>,
    ) -> ContractResult<StateResponse> {
        Ok(StateResponse::Liquidated())
    }

    fn on_time_alarm(
        self,
        _deps: Deps<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        super::ignore_msg(self)
    }
    fn on_price_alarm(
        self,
        _deps: Deps<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        super::ignore_msg(self)
    }
}
