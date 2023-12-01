use serde::{Deserialize, Serialize};

use currencies::Lpns;
use dex::Enterable;
use finance::coin::IntoDTO;
use platform::{bank, batch::Emitter, message::Response as MessageResponse};
use sdk::cosmwasm_std::{
    Coin as CwCoin, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, Timestamp,
};

use crate::{
    api::{DownpaymentCoin, PositionClose, StateResponse},
    contract::{
        cmd::{LiquidationStatus, LiquidationStatusCmd, OpenLoanRespResult, ValidatePayment},
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

    fn try_repay(
        self,
        querier: QuerierWrapper<'_>,
        env: &Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        // TODO: avoid clone
        let response = bank::may_received::<Lpns, _>(info.funds.clone(), IntoDTO::<Lpns>::new());
        match response {
            Some(may_payment) => {
                let payment = may_payment.unwrap();
                debug_assert!(payment.ticker() == self.lease.lease.loan.lpp().currency());
                repay::repay(self.lease, payment, env, querier)
            }
            None => self.validate_and_buy(info.funds, env.block.time, querier),
        }
    }

    fn try_on_price_alarm(
        self,
        querier: QuerierWrapper<'_>,
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
        querier: QuerierWrapper<'_>,
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

    fn try_on_alarm(self, querier: QuerierWrapper<'_>, env: &Env) -> ContractResult<Response> {
        let time_alarms_ref = self.lease.lease.time_alarms.clone();
        let oracle_ref = self.lease.lease.oracle.clone();
        let liquidation_status = self.lease.lease.clone().execute(
            LiquidationStatusCmd::new(env.block.time, &time_alarms_ref, &oracle_ref),
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

    fn validate_and_buy(
        self,
        cw_amount: Vec<CwCoin>,
        now: Timestamp,
        querier: QuerierWrapper<'_>,
    ) -> ContractResult<Response> {
        self.lease
            .lease
            .clone()
            .execute(ValidatePayment::new(cw_amount, now), querier)
            .and_then(|payment| {
                let buy_lpn = buy_lpn::start(self.lease, payment);
                buy_lpn
                    .enter(now, querier)
                    .map(|batch| Response::from(batch, BuyLpnState::from(buy_lpn)))
                    .map_err(Into::into)
            })
    }
}

impl Handler for Active {
    fn state(self, now: Timestamp, querier: QuerierWrapper<'_>) -> ContractResult<StateResponse> {
        super::lease_state(self.lease, None, now, querier)
    }

    fn repay(
        self,
        deps: &mut DepsMut<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        self.try_repay(deps.querier, &env, info)
    }

    fn close_position(
        self,
        spec: PositionClose,
        deps: &mut DepsMut<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        access_control::check(&self.lease.lease.customer, &info.sender)
            .map_err(Into::into)
            .and_then(|()| customer_close::start(spec, self.lease, &env, deps.querier))
    }

    fn on_time_alarm(
        self,
        deps: Deps<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        self.try_on_time_alarm(deps.querier, &env, info)
    }
    fn on_price_alarm(
        self,
        deps: Deps<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        self.try_on_price_alarm(deps.querier, &env, info)
    }
    fn heal(self, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        let lease_addr = self.lease.lease.addr.clone();
        balance::balance(
            &lease_addr,
            self.lease.lease.loan.lpp().currency(),
            deps.querier,
        )
        .and_then(|balance| {
            if balance.is_zero() {
                Err(ContractError::InconsistencyNotDetected())
            } else {
                {
                    repay::repay(self.lease, balance, &env, deps.querier)
                }
            }
        })
    }
}
