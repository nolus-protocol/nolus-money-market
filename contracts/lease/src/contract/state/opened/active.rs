use serde::{Deserialize, Serialize};

use currency::{lpn::Lpns, payment::PaymentGroup};
use dex::Enterable;
use finance::{coin::IntoDTO, liability::Zone};
use platform::{
    bank,
    batch::{Batch, Emitter},
    message::Response as MessageResponse,
};
use sdk::cosmwasm_std::{Deps, DepsMut, Env, MessageInfo, QuerierWrapper, Timestamp};

use crate::{
    api::{DownpaymentCoin, LpnCoin, StateResponse},
    contract::{
        cmd::{
            LiquidationDTO, LiquidationStatus, LiquidationStatusCmd, OpenLoanRespResult,
            PartialCloseFn, Repay, RepayEmitter, RepayFn, RepayResult, Repayable,
        },
        state::{event::LiquidationEmitter, liquidated, paid, Handler, Response},
        Lease,
    },
    error::{ContractError, ContractResult},
};

use super::{
    balance,
    event::{self, PaymentEmitter},
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
        try_repay_lease(
            lease,
            RepayFn {},
            payment,
            PaymentEmitter::new(env),
            env,
            querier,
        )
    }

    pub(in crate::contract::state::opened) fn try_liquidate(
        lease: Lease,
        liquidation: LiquidationDTO,
        liquidation_lpn: LpnCoin,
        querier: &QuerierWrapper<'_>,
        env: &Env,
    ) -> ContractResult<Response> {
        match liquidation {
            LiquidationDTO::Partial {
                amount: _,
                cause: _,
            } => try_partial_liquidation(lease, liquidation, liquidation_lpn, env, querier),
            LiquidationDTO::Full(_) => {
                try_full_liquidation(lease, liquidation, liquidation_lpn, env, querier)
            }
        }
    }

    fn try_repay(self, deps: Deps<'_>, env: &Env, info: MessageInfo) -> ContractResult<Response> {
        let payment = bank::may_received::<PaymentGroup, _>(
            info.funds.clone(),
            IntoDTO::<PaymentGroup>::new(),
        )
        .ok_or_else(ContractError::NoPaymentError)??;

        if payment.ticker() == self.lease.lease.loan.lpp().currency() {
            // TODO once refacture CoinDTO and Group convert to LpnCoin instead
            let payment_lpn = bank::may_received::<Lpns, _>(info.funds, IntoDTO::<Lpns>::new())
                .ok_or_else(ContractError::NoPaymentError)??;

            Self::try_repay_lpn(self.lease, payment_lpn, &deps.querier, env)
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
        // TODO ref. the TODO in try_on_time_alarm
        if !self.lease.lease.oracle.owned_by(&info.sender) {
            return Err(ContractError::Unauthorized(
                access_control::error::Error::Unauthorized {},
            ));
        }

        self.try_on_alarm(querier, env)
    }

    fn try_on_time_alarm(
        self,
        querier: &QuerierWrapper<'_>,
        env: &Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        // TODO define a trait 'RestrictedResource' with 'fn owner(&Addr) -> bool'
        // and move this check to the 'access_control' package
        if !self.lease.lease.time_alarms.owned_by(&info.sender) {
            return Err(ContractError::Unauthorized(
                access_control::error::Error::Unauthorized {},
            ));
        }

        self.try_on_alarm(querier, env)
    }

    fn try_on_alarm(self, querier: &QuerierWrapper<'_>, env: &Env) -> ContractResult<Response> {
        let liquidation_status = self.lease.execute(
            LiquidationStatusCmd::new(
                env.block.time,
                &self.lease.lease.time_alarms,
                &self.lease.lease.oracle,
            ),
            querier,
        )?;

        match liquidation_status {
            LiquidationStatus::NoDebt => Ok(Response::no_msgs(self)),
            LiquidationStatus::NewAlarms {
                current_liability,
                alarms,
            } => Ok(Response::from(
                alarms_resp(&self.lease, current_liability, alarms),
                self,
            )),
            LiquidationStatus::NeedLiquidation(liquidation) => {
                let start_liq = MessageResponse::messages_with_events(
                    Default::default(),
                    event::emit_liquidation_start(&self.lease.lease, &liquidation),
                );
                start_liquidation(self.lease, liquidation, start_liq, env, querier)
            }
        }
    }
}

fn try_partial_liquidation(
    lease: Lease,
    liquidation: LiquidationDTO,
    liquidation_lpn: LpnCoin,
    env: &Env,
    querier: &QuerierWrapper<'_>,
) -> ContractResult<Response> {
    let liquidation_amount = liquidation.amount(&lease.lease).clone();
    try_repay_lease(
        lease,
        PartialCloseFn::new(liquidation_amount.clone()),
        liquidation_lpn,
        LiquidationEmitter::new(liquidation, liquidation_amount, env),
        env,
        querier,
    )
}

fn try_repay_lease<RepayableT, EmitterT>(
    lease: Lease,
    repay_fn: RepayableT,
    amount: LpnCoin,
    emitter_fn: EmitterT,
    env: &Env,
    querier: &QuerierWrapper<'_>,
) -> Result<platform::state_machine::Response<crate::contract::state::State>, ContractError>
where
    RepayableT: Repayable,
    EmitterT: RepayEmitter,
{
    let profit = lease.lease.loan.profit().clone();
    let price_alarms = lease.lease.oracle.clone();
    let time_alarms = lease.lease.time_alarms.clone();
    let (
        lease,
        RepayResult {
            response,
            loan_paid,
            liquidation,
        },
    ) = lease.update(
        Repay::new(
            repay_fn,
            amount,
            env.block.time,
            emitter_fn,
            profit,
            time_alarms,
            price_alarms,
        ),
        querier,
    )?;

    match liquidation {
        LiquidationStatus::NoDebt => Ok(finish_repay(loan_paid, response, lease)),
        LiquidationStatus::NewAlarms {
            current_liability,
            alarms,
        } => {
            let response = alarms_resp(&lease, current_liability, alarms).merge_with(response);
            Ok(finish_repay(loan_paid, response, lease))
        }
        LiquidationStatus::NeedLiquidation(liquidation) => {
            start_liquidation(lease, liquidation, response, env, querier)
        }
    }
}

fn try_full_liquidation(
    lease: Lease,
    liquidation: LiquidationDTO,
    liquidation_lpn: LpnCoin,
    env: &Env,
    querier: &QuerierWrapper<'_>,
) -> ContractResult<Response> {
    let liquidated = liquidated::Liquidated::default();
    liquidated
        .enter_state(
            lease,
            (liquidation, liquidation_lpn),
            env.block.time,
            env,
            querier,
        )
        .map(|response| Response::from(response, liquidated))
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
        .map(|swap_msg| curr_request_response.merge_with(swap_msg))
        .map(|start_liq| Response::from(start_liq, SellAssetState::from(start_liquidaion)))
        .map_err(Into::into)
}

fn alarms_resp(lease: &Lease, current_liability: Zone, alarms: Batch) -> MessageResponse {
    if let Some(events) = current_liability
        .low()
        .map(|low_level| event::emit_liquidation_warning(&lease.lease, &low_level))
    {
        MessageResponse::messages_with_events(alarms, events)
    } else {
        MessageResponse::messages_only(alarms)
    }
}

fn finish_repay(loan_paid: bool, repay_response: MessageResponse, lease: Lease) -> Response {
    if loan_paid {
        Response::from(repay_response, paid::Active::new(lease))
    } else {
        Response::from(repay_response, Active::new(lease))
    }
}

impl Handler for Active {
    fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> ContractResult<StateResponse> {
        super::lease_state(self.lease, None, now, querier)
    }

    fn repay(
        self,
        deps: &mut DepsMut<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        self.try_repay(deps.as_ref(), &env, info)
    }
    fn on_time_alarm(
        self,
        deps: Deps<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        self.try_on_time_alarm(&deps.querier, &env, info)
    }
    fn on_price_alarm(
        self,
        deps: Deps<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        self.try_on_price_alarm(&deps.querier, &env, info)
    }
    fn heal(self, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        let lease_addr = self.lease.lease.addr.clone();
        balance::balance(
            &lease_addr,
            self.lease.lease.loan.lpp().currency(),
            &deps.querier,
        )
        .and_then(|balance| {
            if balance.is_zero() {
                Err(ContractError::InconsistencyNotDetected())
            } else {
                Self::try_repay_lpn(self.lease, balance, &deps.querier, &env)
            }
        })
    }
}
