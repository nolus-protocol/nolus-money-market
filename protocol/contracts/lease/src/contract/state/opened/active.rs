use currency::CurrencyDef;
use serde::{Deserialize, Serialize};

use dex::Enterable;
use finance::{coin::IntoDTO, duration::Duration};
use platform::{bank, batch::Emitter, message::Response as MessageResponse};
use sdk::cosmwasm_std::{Coin as CwCoin, Env, MessageInfo, QuerierWrapper, Timestamp};
use timealarms::stub::TimeAlarmDelivery;

use crate::{
    api::{
        DownpaymentCoin,
        position::{ClosePolicyChange, PositionClose},
        query::{StateResponse, opened::Status},
    },
    contract::{
        Lease,
        cmd::{
            ChangeClosePolicy, CloseStatusCmd, CloseStatusDTO, ObtainPayment, OpenLoanRespResult,
        },
        state::{
            Handler, Response,
            opened::permission::{
                ChangeClosePolicyPermission, ClosePositionPermission, PriceAlarmDelivery,
            },
        },
    },
    error::{ContractError, ContractResult},
    finance::{LpnCurrencies, LpnCurrency},
};

use super::{
    alarm, balance,
    close::sell_asset::{customer_close, liquidation},
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
    pub(in super::super) fn new(lease: Lease) -> Self {
        Self { lease }
    }

    pub(in super::super) fn emit_opened(
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
        let may_lpn_payment = bank::may_received(&info.funds, IntoDTO::<LpnCurrencies>::new());
        match may_lpn_payment {
            Some(payment) => {
                debug_assert!(payment.of_currency_dto(LpnCurrency::dto()).is_ok());
                repay::repay(self.lease, payment, env, querier)
            }
            None => self.start_swap(info.funds, env.block.time, querier),
        }
    }

    // Access permission check should already have been done!
    pub(super) fn assess_close_status(
        self,
        querier: QuerierWrapper<'_>,
        env: &Env,
    ) -> ContractResult<Response> {
        self.try_on_alarm(querier, env)
    }

    fn try_on_alarm(self, querier: QuerierWrapper<'_>, env: &Env) -> ContractResult<Response> {
        let time_alarms_ref = self.lease.lease.time_alarms.clone();
        let oracle_ref = self.lease.lease.oracle.clone();
        let close_status = self.lease.lease.clone().execute(
            CloseStatusCmd::new(&env.block.time, &time_alarms_ref, &oracle_ref),
            querier,
        )?;

        match close_status {
            CloseStatusDTO::Paid => {
                unimplemented!("an Active Opened Lease should always have some due amount")
            }
            CloseStatusDTO::None {
                current_liability,
                alarms,
            } => Ok(Response::from(
                alarm::build_resp(&self.lease, current_liability, alarms),
                self,
            )),
            CloseStatusDTO::NeedLiquidation(liquidation) => liquidation::start(
                self.lease,
                liquidation,
                MessageResponse::default(),
                env,
                querier,
            ),
            CloseStatusDTO::CloseAsked(strategy) => {
                customer_close::auto_start(strategy, self.lease, env, querier)
            }
        }
    }

    fn start_swap(
        self,
        cw_amount: Vec<CwCoin>,
        now: Timestamp,
        querier: QuerierWrapper<'_>,
    ) -> ContractResult<Response> {
        self.lease
            .lease
            .clone()
            .execute(ObtainPayment::new(cw_amount), querier)
            .and_then(|payment| {
                let buy_lpn = buy_lpn::start(self.lease, payment);
                buy_lpn
                    .enter(now, querier)
                    .map(|batch| Response::from(batch, BuyLpnState::from(buy_lpn)))
                    .map_err(Into::into)
            })
    }
}

impl From<Active> for Lease {
    fn from(value: Active) -> Self {
        value.lease
    }
}

impl Handler for Active {
    fn state(
        self,
        now: Timestamp,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> ContractResult<StateResponse> {
        super::lease_state(self.lease, Status::Idle, now, due_projection, querier)
    }

    fn repay(
        self,
        querier: QuerierWrapper<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        self.try_repay(querier, &env, info)
    }

    fn change_close_policy(
        self,
        change: ClosePolicyChange,
        querier: QuerierWrapper<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        access_control::check(
            &ChangeClosePolicyPermission::new(&self.lease.lease.customer),
            &info,
        )
        .map_err(Into::into)
        .and_then(|()| {
            let profit = self.lease.lease.loan.profit().clone();
            let time_alarms = self.lease.lease.time_alarms.clone();
            let oracle_ref = self.lease.lease.oracle.clone();
            let reserve = self.lease.lease.reserve.clone();
            self.lease.update(
                ChangeClosePolicy::new(
                    change,
                    &env.block.time,
                    profit,
                    time_alarms,
                    &oracle_ref,
                    reserve,
                ),
                querier,
            )
        })
        .and_then(|(lease, close_status)| match close_status {
            CloseStatusDTO::Paid => {
                unimplemented!("only changing an Active Opened Lease is permitted")
            }
            CloseStatusDTO::None {
                current_liability,
                alarms,
            } => Ok(Response::from(
                alarm::build_resp(&lease, current_liability, alarms),
                Self::new(lease),
            )),
            CloseStatusDTO::CloseAsked(_) => unimplemented!(
                "triggering a close with a policy change should have already resulted in an error"
            ),
            CloseStatusDTO::NeedLiquidation(liquidation) => liquidation::start(
                lease,
                liquidation,
                MessageResponse::default(),
                &env,
                querier,
            ),
        })
    }

    fn close_position(
        self,
        spec: PositionClose,
        querier: QuerierWrapper<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        access_control::check(
            &ClosePositionPermission::new(&self.lease.lease.customer),
            &info,
        )
        .map_err(Into::into)
        .and_then(|()| customer_close::start(spec, self.lease, &env, querier))
    }

    fn on_time_alarm(
        self,
        querier: QuerierWrapper<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        access_control::check(
            &TimeAlarmDelivery::new(&self.lease.lease.time_alarms),
            &info,
        )?;

        self.try_on_alarm(querier, &env)
    }

    fn on_price_alarm(
        self,
        querier: QuerierWrapper<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        access_control::check(&PriceAlarmDelivery::new(&self.lease.lease.oracle), &info)?;

        self.try_on_alarm(querier, &env)
    }

    fn heal(
        self,
        querier: QuerierWrapper<'_>,
        env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        let lease_addr = self.lease.lease.addr.clone();
        balance::lpn_balance(&lease_addr, querier).and_then(|balance| {
            if balance.is_zero() {
                Err(ContractError::InconsistencyNotDetected())
            } else {
                repay::repay(self.lease, balance, &env, querier)
            }
        })
    }
}
