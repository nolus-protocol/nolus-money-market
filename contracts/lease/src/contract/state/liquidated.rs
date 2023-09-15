use platform::message::Response as MessageResponse;
use serde::{Deserialize, Serialize};

use sdk::cosmwasm_std::{Deps, Env, MessageInfo, QuerierWrapper, Timestamp};

use crate::{
    api::{LpnCoin, StateResponse},
    contract::{
        cmd::{FullLiquidation, FullLiquidationResult, LiquidationDTO, RepayEmitter},
        Lease,
    },
    error::ContractResult,
    lease::with_lease,
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
        let lease_addr = lease.lease.addr.clone();
        let liquidation = liquidation_descr.0;
        let liquidation_lpn = liquidation_descr.1;
        let liquidation_amount = liquidation.amount(&lease.lease).clone();
        let customer = lease.lease.customer.clone();
        let profit = lease.lease.loan.profit().clone();

        // TODO define a fn similar to `contract::Lease::execute`
        with_lease::execute(
            lease.lease,
            FullLiquidation::new(liquidation_lpn, now, profit),
            querier,
        )
        .map(|FullLiquidationResult { receipt, messages }| {
            // TODO move event emitting into an emitFn passed to the `FullLiquidation`
            MessageResponse::messages_with_events(
                messages,
                LiquidationEmitter::new(liquidation, liquidation_amount, env)
                    .emit(&lease_addr, &receipt),
            )
        })
        .and_then(|liquidation_response| {
            lease
                .finalizer
                .notify(customer)
                .map(|finalizer_msgs| liquidation_response.merge_with(finalizer_msgs))
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
