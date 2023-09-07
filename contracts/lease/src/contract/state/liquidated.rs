use finance::liability::Cause;
use platform::{batch::Emitter, message::Response as MessageResponse};
use profit::stub::ProfitRef;
use serde::{Deserialize, Serialize};

use sdk::cosmwasm_std::{Addr, Deps, Env, MessageInfo, QuerierWrapper, Timestamp};

use crate::{
    api::{LeaseCoin, LpnCoin, StateResponse},
    contract::{
        cmd::{FullLiquidation, FullLiquidationResult, LiquidationDTO, ReceiptDTO},
        state::event,
        Lease,
    },
    error::ContractResult,
    lease::with_lease,
};

use super::{Handler, Response};

#[derive(Serialize, Deserialize, Default)]
pub struct Liquidated {}

impl Liquidated {
    pub(super) fn enter_state(
        &self,
        lease: Lease,
        liquidation_descr: (LiquidationDTO, LpnCoin),
        now: Timestamp,
        profit: ProfitRef,
        env: &Env,
        querier: &QuerierWrapper<'_>,
    ) -> ContractResult<MessageResponse> {
        let lease_addr = lease.lease.addr.clone();
        let liquidation = liquidation_descr.0;
        let liquidation_lpn = liquidation_descr.1;
        let liquidation_amount = liquidation.amount(&lease.lease).clone();
        let customer = lease.lease.customer.clone();

        with_lease::execute(
            lease.lease,
            FullLiquidation::new(liquidation_lpn, now, profit),
            querier,
        )
        .map(|FullLiquidationResult { receipt, messages }| {
            MessageResponse::messages_with_events(
                messages,
                self.emit_ok(
                    env,
                    &lease_addr,
                    &receipt,
                    liquidation.cause(),
                    &liquidation_amount,
                ),
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

    fn emit_ok(
        &self,
        env: &Env,
        lease_addr: &Addr,
        receipt: &ReceiptDTO,
        liquidation_cause: &Cause,
        liquidation_amount: &LeaseCoin,
    ) -> Emitter {
        event::emit_liquidation(
            env,
            lease_addr,
            receipt,
            liquidation_cause,
            liquidation_amount,
        )
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
