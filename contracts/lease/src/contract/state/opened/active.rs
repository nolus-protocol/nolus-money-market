use currency::payment::PaymentGroup;
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
    api::{DownpaymentCoin, ExecuteMsg, StateQuery, StateResponse},
    contract::{
        cmd::{
            AlarmResult, Close, LeaseState, OpenLoanRespResult, PriceAlarm, Repay, RepayResult,
            TimeAlarm,
        },
        state::{Controller, Response},
    },
    error::{ContractError, ContractResult},
    event::Type,
    lease::{with_lease, IntoDTOResult, LeaseDTO},
};

use super::repay::transfer_out::TransferOut;

#[derive(Serialize, Deserialize)]
pub struct Active {
    lease: LeaseDTO,
    // dex: ConnectionParams,
    // dex_account: HostAccount,
}

impl Active {
    pub(in crate::contract::state) fn new(lease: LeaseDTO) -> Self {
        Self { lease }
    }

    pub(in crate::contract::state) fn enter_state(
        &self,
        batch: Batch,
        env: &Env,
        downpayment: DownpaymentCoin,
        loan: OpenLoanRespResult,
    ) -> Emitter {
        build_emitter(batch, env, &self.lease, loan, downpayment)
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
            ExecuteMsg::Repay() => try_repay(&deps.querier, &env, info, self.lease),
            ExecuteMsg::Close() => {
                let RepayResult { lease, emitter } =
                    try_close(&deps.querier, &env, info, self.lease)?;

                Ok(into_resp(emitter, lease))
            }
            ExecuteMsg::PriceAlarm() => {
                let AlarmResult {
                    response,
                    lease_dto: lease_updated,
                } = try_on_price_alarm(&deps.querier, &env, info, self.lease)?;

                Ok(into_resp(response, lease_updated))
            }
            ExecuteMsg::TimeAlarm(_block_time) => {
                let AlarmResult {
                    response,
                    lease_dto: lease_updated,
                } = try_on_time_alarm(&deps.querier, &env, info, self.lease)?;

                Ok(into_resp(response, lease_updated))
            }
        }
    }

    fn query(self, deps: Deps, env: Env, _msg: StateQuery) -> ContractResult<StateResponse> {
        // TODO think on taking benefit from having a LppView trait
        with_lease::execute(
            self.lease,
            LeaseState::new(env.block.time),
            &env.contract.address,
            &deps.querier,
        )
    }
}

fn try_repay(
    querier: &QuerierWrapper,
    env: &Env,
    info: MessageInfo,
    lease: LeaseDTO,
) -> ContractResult<Response> {
    let payment = bank::may_received::<PaymentGroup, _>(info.funds, IntoDTO::<PaymentGroup>::new())
        .ok_or_else(ContractError::NoPaymentError)??;
    if payment.ticker() == lease.loan.lpp().currency() {
        let RepayResult {
            lease: lease_updated,
            emitter,
        } = with_lease::execute(
            lease,
            Repay::new(payment, env),
            &env.contract.address,
            querier,
        )?;
        Ok(into_resp(emitter, lease_updated))
    } else {
        let this_addr = env.contract.address.clone();
        let next_state = TransferOut::new(lease, payment);
        let batch = next_state.enter_state(this_addr, env.block.time)?;
        Ok(Response::from(batch, next_state))
    }
}

fn try_close(
    querier: &QuerierWrapper,
    env: &Env,
    info: MessageInfo,
    lease: LeaseDTO,
) -> ContractResult<RepayResult> {
    //TODO Move RepayResult into this layer, rename to, for example, ExecuteResult
    // and refactor try_* to return it
    // Take the emitting out of the commands layer
    let account = bank::my_account(env, querier);
    let IntoDTOResult { lease, batch } = with_lease::execute(
        lease,
        Close::new(&info.sender, account),
        &env.contract.address,
        querier,
    )?;

    let emitter = batch
        .into_emitter(Type::Close)
        .emit("id", env.contract.address.clone())
        .emit_tx_info(env);

    Ok(RepayResult { emitter, lease })
}

fn try_on_price_alarm(
    querier: &QuerierWrapper,
    env: &Env,
    info: MessageInfo,
    lease: LeaseDTO,
) -> ContractResult<AlarmResult> {
    with_lease::execute(
        lease,
        PriceAlarm::new(env, &info.sender, env.block.time),
        &env.contract.address,
        querier,
    )
}

fn try_on_time_alarm(
    querier: &QuerierWrapper,
    env: &Env,
    info: MessageInfo,
    lease: LeaseDTO,
) -> ContractResult<AlarmResult> {
    with_lease::execute(
        lease,
        TimeAlarm::new(env, &info.sender, env.block.time),
        &env.contract.address,
        querier,
    )
}

fn into_resp<R>(resp: R, lease: LeaseDTO) -> Response
where
    R: Into<CwResponse>,
{
    Response::from(resp, Active { lease })
}

fn build_emitter(
    batch: Batch,
    env: &Env,
    dto: &LeaseDTO,
    loan: OpenLoanRespResult,
    downpayment: DownpaymentCoin,
) -> Emitter {
    batch
        .into_emitter(Type::Open)
        .emit_tx_info(env)
        .emit("id", env.contract.address.clone())
        .emit("customer", dto.customer.clone())
        .emit_percent_amount(
            "air",
            loan.annual_interest_rate + dto.loan.annual_margin_interest(),
        )
        .emit("currency", dto.amount.ticker())
        .emit("loan-pool-id", dto.loan.lpp().addr())
        .emit_coin_dto("loan", loan.principal)
        .emit_coin_dto("downpayment", downpayment)
}
