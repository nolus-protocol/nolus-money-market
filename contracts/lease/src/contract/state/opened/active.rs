use serde::{Deserialize, Serialize};

use currency::{lpn::Lpns, payment::PaymentGroup};
use dex::Enterable;
use finance::coin::IntoDTO;
use platform::{bank, batch::Emitter, message::Response as MessageResponse};
use sdk::cosmwasm_std::{Deps, DepsMut, Env, MessageInfo, QuerierWrapper, Timestamp};

use crate::{
    api::{DownpaymentCoin, PositionClose, StateResponse},
    contract::{
        cmd::{LiquidationStatus, LiquidationStatusCmd, OpenLoanRespResult},
        state::{Handler, Response},
        Lease,
    },
    error::{ContractError, ContractResult},
};

use super::{
    alarm, balance,
    close::{customer_close, liquidation},
    event,
    repay::{
        self,
        buy_lpn::{self, DexState as BuyLpnState},
    },
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

    fn try_repay(self, deps: Deps<'_>, env: &Env, info: MessageInfo) -> ContractResult<Response> {
        let payment = bank::may_received::<PaymentGroup, _>(
            info.funds.clone(),
            IntoDTO::<PaymentGroup>::new(),
        )
        .ok_or_else(ContractError::NoPaymentError)??;

        if payment.ticker() == self.lease.lease.loan.lpp().currency() {
            // TODO once refacture CoinDTO and Group convert to LpnCoin instead
            bank::may_received::<Lpns, _>(info.funds, IntoDTO::<Lpns>::new())
                .ok_or_else(ContractError::NoPaymentError)?
                .map_err(Into::into)
                .and_then(|payment_lpn| repay::repay(self.lease, payment_lpn, env, &deps.querier))
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
                alarm::build_resp(&self.lease, current_liability, alarms),
                self,
            )),
            LiquidationStatus::NeedLiquidation(liquidation) => liquidation::start(
                self.lease,
                liquidation,
                MessageResponse::default(),
                env,
                querier,
            ),
        }
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

    fn close_position(
        self,
        spec: PositionClose,
        deps: &mut DepsMut<'_>,
        env: Env,
    ) -> ContractResult<Response> {
        customer_close::start(spec, self.lease, &env, &deps.querier)
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
                {
                    repay::repay(self.lease, balance, &env, &deps.querier)
                }
            }
        })
    }
}
