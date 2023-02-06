use currency::{lpn::Lpns, payment::PaymentGroup};
use finance::coin::IntoDTO;
use serde::{Deserialize, Serialize};

use platform::{
    bank::{self},
    batch::{Batch, Emit, Emitter},
};
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{Deps, DepsMut, Env, MessageInfo, QuerierWrapper},
};

use crate::{
    api::{DownpaymentCoin, ExecuteMsg, LpnCoin, StateQuery, StateResponse},
    contract::{
        cmd::{AlarmResult, OpenLoanRespResult, PriceAlarm, Repay, RepayResult, TimeAlarm},
        state::{paid, Controller, Response},
        Lease,
    },
    dex::Account,
    error::{ContractError, ContractResult},
    event::Type,
    lease::{with_lease, LeaseDTO},
};

use super::repay::transfer_out::TransferOut;

#[derive(Serialize, Deserialize)]
pub struct Active {
    lease: Lease,
}

impl Active {
    pub(in crate::contract::state) fn new(lease: Lease) -> Self {
        Self { lease }
    }

    pub(in crate::contract::state) fn enter_state(
        &self,
        batch: Batch,
        env: &Env,
        downpayment: DownpaymentCoin,
        loan: OpenLoanRespResult,
    ) -> Emitter {
        build_emitter(batch, env, &self.lease.lease, loan, downpayment)
    }

    pub(in crate::contract::state::opened) fn try_repay_lpn(
        lease: Lease,
        payment: LpnCoin,
        querier: &QuerierWrapper,
        env: &Env,
    ) -> ContractResult<Response> {
        // TODO return ContractResult<(RepayReceipt, Batch)>
        // TODO Move RepayResult into this layer, rename to, for example, ExecuteResult
        // and refactor try_* to return it
        let RepayResult {
            lease: lease_updated,
            paid,
            emitter,
        } = with_lease::execute(lease.lease, Repay::new(payment, env), querier)?;

        let new_lease = Lease {
            lease: lease_updated,
            dex: lease.dex,
        };
        let resp = if paid {
            Response::from(emitter, paid::Active::new(new_lease))
        } else {
            Response::from(emitter, Active::new(new_lease))
        };
        Ok(resp)
    }

    fn try_repay(
        self,
        querier: &QuerierWrapper,
        env: &Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        let payment = bank::may_received::<PaymentGroup, _>(
            info.funds.clone(),
            IntoDTO::<PaymentGroup>::new(),
        )
        .ok_or_else(ContractError::NoPaymentError)??;
        if payment.ticker() == self.lease.lease.loan.lpp().currency() {
            // TODO once refacture CoinDTO and Group convert to LpnCoin instead
            let payment_lpn = bank::may_received::<Lpns, _>(info.funds, IntoDTO::<Lpns>::new())
                .ok_or_else(ContractError::NoPaymentError)??;

            Self::try_repay_lpn(self.lease, payment_lpn, querier, env)
        } else {
            let next_state = TransferOut::new(self.lease, payment);
            let batch = next_state.enter_state(env.block.time)?;
            Ok(Response::from(batch, next_state))
        }
    }

    fn try_on_price_alarm(
        self,
        querier: &QuerierWrapper,
        env: &Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        let AlarmResult {
            response,
            lease_dto: lease_updated,
        } = with_lease::execute(
            self.lease.lease,
            PriceAlarm::new(env, &info.sender, env.block.time),
            querier,
        )?;
        Ok(into_updated_active(lease_updated, self.lease.dex, response))
    }

    fn try_on_time_alarm(
        self,
        querier: &QuerierWrapper,
        env: &Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        let AlarmResult {
            response,
            lease_dto: lease_updated,
        } = with_lease::execute(
            self.lease.lease,
            TimeAlarm::new(env, &info.sender, env.block.time),
            querier,
        )?;
        Ok(into_updated_active(lease_updated, self.lease.dex, response))
    }
}

impl Controller for Active {
    fn execute(
        self,
        deps: &mut DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> ContractResult<Response> {
        match msg {
            ExecuteMsg::Repay() => self.try_repay(&deps.querier, &env, info),
            ExecuteMsg::Close() => todo!("fail"),
            ExecuteMsg::PriceAlarm() => self.try_on_price_alarm(&deps.querier, &env, info),
            ExecuteMsg::TimeAlarm(_block_time) => self.try_on_time_alarm(&deps.querier, &env, info),
        }
    }

    fn query(self, deps: Deps, env: Env, _msg: StateQuery) -> ContractResult<StateResponse> {
        super::query(self.lease.lease, None, &deps, &env)
    }
}

fn build_emitter(
    batch: Batch,
    env: &Env,
    lease: &LeaseDTO,
    loan: OpenLoanRespResult,
    downpayment: DownpaymentCoin,
) -> Emitter {
    batch
        .into_emitter(Type::Open)
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

fn into_updated_active<R>(updated_dto: LeaseDTO, dex: Account, resp: R) -> Response
where
    R: Into<CwResponse>,
{
    let lease = Lease {
        lease: updated_dto,
        dex,
    };
    Response::from(resp, Active { lease })
}
