use finance::liability::Cause;
use platform::batch::Emitter;
use profit::stub::ProfitRef;
use serde::{Deserialize, Serialize};

use sdk::cosmwasm_std::{Addr, Deps, Env, MessageInfo, QuerierWrapper, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{LeaseCoin, LpnCoin, StateResponse},
    contract::{
        cmd::{FullLiquidation, FullLiquidationResult, ReceiptDTO},
        state::event,
    },
    error::ContractResult,
    lease::{self, LeaseDTO},
};

use super::{Handler, Response};

#[derive(Serialize, Deserialize, Default)]
pub struct Liquidated {}

impl Liquidated {
    pub(super) fn enter_state(
        &self,
        lease: LeaseDTO,
        liquidation_lpn: LpnCoin,
        now: Timestamp,
        profit: ProfitRef,
        time_alarms: TimeAlarmsRef,
        querier: &QuerierWrapper<'_>,
    ) -> ContractResult<FullLiquidationResult> {
        lease::with_lease::execute(
            lease,
            FullLiquidation::new(liquidation_lpn, now, profit, time_alarms),
            querier,
        )
    }

    pub(super) fn emit_ok(
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
