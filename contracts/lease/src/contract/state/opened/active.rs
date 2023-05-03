use serde::{Deserialize, Serialize};

use currency::{lpn::Lpns, payment::PaymentGroup};
use dex::Enterable;
use finance::coin::IntoDTO;
use platform::{bank, batch::Emitter, message::Response as MessageResponse};
use sdk::cosmwasm_std::{Deps, DepsMut, Env, MessageInfo, QuerierWrapper, Timestamp};

use crate::{
    api::{DownpaymentCoin, ExecuteMsg, LpnCoin, StateResponse},
    contract::{
        cmd::{
            Liquidate, LiquidateResult, LiquidationDTO, LiquidationStatusCmd,
            LiquidationStatusCmdResult, OpenLoanRespResult, Repay, RepayResult,
        },
        state::{handler, paid, Handler, Response},
        Contract, Lease,
    },
    error::{ContractError, ContractResult},
    lease::with_lease,
};

use super::{
    event,
    liquidation::sell_asset::{self, DexState as SellAssetState},
    repay::buy_lpn::{self, DexState as BuyLpnState},
};

#[derive(Serialize, Deserialize)]
pub struct Active {
    lease: Lease,
}

impl Active {
    pub(in crate::contract::state) fn new(lease: Lease) -> Self {
        Self { lease }
    }

    pub(in crate::contract::state) fn emit_opened(
        &self,
        env: &Env,
        downpayment: DownpaymentCoin,
        loan: OpenLoanRespResult,
    ) -> Emitter {
        event::emit_lease_opened(env, &self.lease.lease, loan, downpayment)
    }

    pub(in crate::contract::state::opened) fn try_repay_lpn(
        lease: Lease,
        payment: LpnCoin,
        querier: &QuerierWrapper<'_>,
        env: &Env,
    ) -> ContractResult<Response> {
        let time_alarms = lease.lease.time_alarms.clone();
        let RepayResult {
            lease: lease_updated,
            receipt,
            messages: repay_messages,
            liquidation,
        } = with_lease::execute(
            lease.lease,
            Repay::new(payment, env.block.time, time_alarms),
            querier,
        )?;

        let repay_response = MessageResponse::messages_with_events(
            repay_messages,
            event::emit_payment(env, &lease_updated, &receipt),
        );

        let lease = Lease::new(lease_updated, lease.dex);
        if let Some(liquidation) = liquidation {
            Self::start_liquidation(lease, liquidation, repay_response, env, querier)
        } else {
            Ok(finish_no_liquidation(receipt.close, repay_response, lease))
        }
    }

    pub(in crate::contract::state::opened) fn try_liquidate(
        lease: Lease,
        liquidation: LiquidationDTO,
        liquidation_lpn: LpnCoin,
        querier: &QuerierWrapper<'_>,
        env: &Env,
    ) -> ContractResult<Response> {
        let time_alarms = lease.lease.time_alarms.clone();
        let liquidation_asset = liquidation.amount(&lease.lease).clone();
        let LiquidateResult {
            lease: lease_updated,
            receipt,
            messages: liquidate_messages,
            liquidation: next_liquidation,
        } = with_lease::execute(
            lease.lease,
            Liquidate::new(
                liquidation_asset,
                liquidation_lpn,
                env.block.time,
                time_alarms,
            ),
            querier,
        )?;

        let liquidate_response = MessageResponse::messages_with_events(
            liquidate_messages,
            event::emit_liquidation(env, &lease_updated, &receipt, &liquidation),
        );

        let lease = Lease::new(lease_updated, lease.dex);
        if let Some(liquidation) = next_liquidation {
            Self::start_liquidation(lease, liquidation, liquidate_response, env, querier)
        } else {
            Ok(finish_no_liquidation(
                receipt.close,
                liquidate_response,
                lease,
            ))
        }
    }

    fn try_repay(self, deps: Deps<'_>, env: Env, info: MessageInfo) -> ContractResult<Response> {
        let payment = bank::may_received::<PaymentGroup, _>(
            info.funds.clone(),
            IntoDTO::<PaymentGroup>::new(),
        )
        .ok_or_else(ContractError::NoPaymentError)??;

        if payment.ticker() == self.lease.lease.loan.lpp().currency() {
            // TODO once refacture CoinDTO and Group convert to LpnCoin instead
            let payment_lpn = bank::may_received::<Lpns, _>(info.funds, IntoDTO::<Lpns>::new())
                .ok_or_else(ContractError::NoPaymentError)??;

            Self::try_repay_lpn(self.lease, payment_lpn, &deps.querier, &env)
        } else {
            let start_buy_lpn = buy_lpn::start(self.lease, payment);
            start_buy_lpn
                .enter(env.block.time, &deps.querier)
                .map(|batch| Response::from(batch, BuyLpnState::from(start_buy_lpn)))
                .map_err(Into::into)
        }
    }

    fn try_on_price_alarm(
        self,
        querier: &QuerierWrapper<'_>,
        env: &Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        if !self.lease.lease.oracle.owned_by(&info.sender) {
            return Err(ContractError::Unauthorized {});
        }

        self.try_on_alarm(querier, env)
    }

    fn try_on_time_alarm(
        self,
        querier: &QuerierWrapper<'_>,
        env: &Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        if !self.lease.lease.time_alarms.owned_by(&info.sender) {
            return Err(ContractError::Unauthorized {});
        }

        self.try_on_alarm(querier, env)
    }

    fn try_on_alarm(self, querier: &QuerierWrapper<'_>, env: &Env) -> ContractResult<Response> {
        let time_alarms = self.lease.lease.time_alarms.clone();
        let liquidation_status = with_lease::execute(
            self.lease.lease.clone(),
            LiquidationStatusCmd::new(env.block.time, time_alarms),
            querier,
        )?;

        match liquidation_status {
            LiquidationStatusCmdResult::NewAlarms {
                current_liability,
                alarms,
            } => {
                let resp = if let Some(events) = current_liability
                    .low()
                    .map(|low_level| event::emit_liquidation_warning(&self.lease.lease, &low_level))
                {
                    MessageResponse::messages_with_events(alarms, events)
                } else {
                    MessageResponse::messages_only(alarms)
                };
                Ok(Response::from(resp, self))
            }
            LiquidationStatusCmdResult::NeedLiquidation(liquidation) => {
                let start_liq = MessageResponse::messages_with_events(
                    Default::default(),
                    event::emit_liquidation_start(&self.lease.lease, &liquidation),
                );
                Self::start_liquidation(self.lease, liquidation, start_liq, env, querier)
            }
        }
    }

    fn start_liquidation(
        lease: Lease,
        liquidation: LiquidationDTO,
        curr_request_response: MessageResponse,
        env: &Env,
        querier: &QuerierWrapper<'_>,
    ) -> ContractResult<Response> {
        let start_liquidaion = sell_asset::start(lease, liquidation);
        start_liquidaion
            .enter(env.block.time, querier)
            .map(|swap_msg| curr_request_response.add_messages(swap_msg))
            .map(|start_liq| Response::from(start_liq, SellAssetState::from(start_liquidaion)))
            .map_err(Into::into)
    }
}

fn finish_no_liquidation(
    lease_closed: bool,
    repay_response: MessageResponse,
    lease: Lease,
) -> Response {
    if lease_closed {
        Response::from(repay_response, paid::Active::new(lease))
    } else {
        Response::from(repay_response, Active::new(lease))
    }
}

impl Handler for Active {
    fn execute(
        self,
        deps: &mut DepsMut<'_>,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> ContractResult<Response> {
        match msg {
            ExecuteMsg::Repay() => self.try_repay(deps.as_ref(), env, info),
            ExecuteMsg::Close() => handler::err("close", deps.api),
            ExecuteMsg::PriceAlarm() => self.try_on_price_alarm(&deps.querier, &env, info),
            ExecuteMsg::TimeAlarm {} => self.try_on_time_alarm(&deps.querier, &env, info),
        }
    }
}

impl Contract for Active {
    fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> ContractResult<StateResponse> {
        super::lease_state(self.lease.lease, None, now, querier)
    }
}
