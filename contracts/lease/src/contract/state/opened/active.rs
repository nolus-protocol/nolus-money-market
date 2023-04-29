use serde::{Deserialize, Serialize};

use currency::{lpn::Lpns, payment::PaymentGroup};
use dex::Enterable;
use finance::coin::IntoDTO;
use platform::{
    bank,
    batch::{Emit, Emitter},
};
use sdk::cosmwasm_std::{Deps, DepsMut, Env, MessageInfo, QuerierWrapper, Timestamp};

use crate::{
    api::{DownpaymentCoin, ExecuteMsg, LpnCoin, StateResponse},
    contract::{
        cmd::{LiquidationStatus, OpenLoanRespResult, Repay, RepayResult},
        state::{handler, paid, Handler, Response},
        Contract, Lease,
    },
    error::{ContractError, ContractResult},
    event::Type,
    lease::{with_lease, LeaseDTO},
};

use super::repay::buy_lpn::{self, DexState};

#[derive(Serialize, Deserialize)]
pub struct Active {
    lease: Lease,
}

impl Active {
    pub(in crate::contract::state) fn new(lease: Lease) -> Self {
        Self { lease }
    }

    pub(in crate::contract::state) fn emit_ok(
        &self,
        env: &Env,
        downpayment: DownpaymentCoin,
        loan: OpenLoanRespResult,
    ) -> Emitter {
        build_emitter(env, &self.lease.lease, loan, downpayment)
    }

    pub(in crate::contract::state::opened) fn try_repay_lpn(
        lease: Lease,
        payment: LpnCoin,
        querier: &QuerierWrapper<'_>,
        env: &Env,
    ) -> ContractResult<Response> {
        let RepayResult {
            lease: lease_updated,
            paid,
            response,
        } = with_lease::execute(lease.lease, Repay::new(payment, env), querier)?;

        let new_lease = Lease {
            lease: lease_updated,
            dex: lease.dex,
        };

        Ok(if paid {
            Response::from(response, paid::Active::new(new_lease))
        } else {
            Response::from(response, Active::new(new_lease))
        })
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
                .map(|batch| Response::from(batch, DexState::from(start_buy_lpn)))
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

        let response = with_lease::execute(
            self.lease.lease.clone(),
            LiquidationStatus::new(env.block.time),
            querier,
        )?;

        Ok(Response::from(response, self))
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

        let response = with_lease::execute(
            self.lease.lease.clone(),
            LiquidationStatus::new(env.block.time),
            querier,
        )?;

        Ok(Response::from(response, self))
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

fn build_emitter(
    env: &Env,
    lease: &LeaseDTO,
    loan: OpenLoanRespResult,
    downpayment: DownpaymentCoin,
) -> Emitter {
    Emitter::of_type(Type::OpenedActive)
        .emit_tx_info(env)
        .emit("id", &lease.addr)
        .emit("customer", lease.customer.clone())
        .emit_percent_amount(
            "air",
            loan.annual_interest_rate + lease.loan.annual_margin_interest(),
        )
        .emit("currency", lease.amount.ticker())
        .emit("loan-pool-id", lease.loan.lpp().addr())
        .emit_coin_dto("loan", loan.principal)
        .emit_coin_dto("downpayment", downpayment)
}
