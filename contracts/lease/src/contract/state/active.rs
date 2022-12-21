use std::fmt::Display;

use lpp::stub::lender::LppLenderRef;
use market_price_oracle::stub::OracleRef;
use serde::{Deserialize, Serialize};

use platform::{
    bank::{self},
    batch::{Batch, Emit, Emitter},
};
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, QuerierWrapper},
};

use crate::{
    api::{DownpaymentCoin, ExecuteMsg, LeaseCoin, NewLeaseForm, StateQuery},
    contract::{
        alarms::{price::PriceAlarm, time::TimeAlarm, AlarmResult},
        close::Close,
        cmd::{LeaseState, OpenLoanRespResult},
        repay::{Repay, RepayResult},
    },
    error::ContractResult,
    event::Type,
    lease::{self, IntoDTOResult, LeaseDTO},
};

use super::{Controller, Response};

#[derive(Serialize, Deserialize)]
pub struct Active {
    lease: LeaseDTO,
}

impl Active {
    pub fn new(
        cw_deps: &DepsMut,
        env: &Env,
        form: NewLeaseForm,
        downpayment: DownpaymentCoin,
        loan: OpenLoanRespResult,
        amount: LeaseCoin,
        deps: (LppLenderRef, OracleRef),
    ) -> ContractResult<(Emitter, Self)> {
        debug_assert_eq!(downpayment.ticker(), loan.principal.ticker());

        let IntoDTOResult { lease, batch } = form.into_lease(
            &env.contract.address,
            env.block.time,
            &amount,
            &cw_deps.querier,
            deps,
        )?;
        let emitter = build_emitter(batch, env, &lease, loan, downpayment);
        Ok((emitter, Self { lease }))
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
        let resp = match msg {
            ExecuteMsg::Repay() => {
                let RepayResult {
                    lease: lease_updated,
                    emitter,
                } = try_repay(&deps.querier, &env, info, self.lease)?;

                into_resp(emitter, lease_updated)
            }
            ExecuteMsg::Close() => {
                let RepayResult { lease, emitter } =
                    try_close(&deps.querier, &env, info, self.lease)?;

                into_resp(emitter, lease)
            }
            ExecuteMsg::PriceAlarm() => {
                let AlarmResult {
                    response,
                    lease_dto: lease_updated,
                } = try_on_price_alarm(&deps.querier, &env, info, self.lease)?;

                into_resp(response, lease_updated)
            }
            ExecuteMsg::TimeAlarm(_block_time) => {
                let AlarmResult {
                    response,
                    lease_dto: lease_updated,
                } = try_on_time_alarm(&deps.querier, &env, info, self.lease)?;

                into_resp(response, lease_updated)
            }
        };
        Ok(resp)
    }

    fn query(self, deps: Deps, env: Env, _msg: StateQuery) -> ContractResult<Binary> {
        // TODO think on taking benefit from having a LppView trait
        lease::execute(
            self.lease,
            LeaseState::new(env.block.time),
            &env.contract.address,
            &deps.querier,
        )
    }
}

impl Display for Active {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("active lease in idle state")
    }
}

fn try_repay(
    querier: &QuerierWrapper,
    env: &Env,
    info: MessageInfo,
    lease: LeaseDTO,
) -> ContractResult<RepayResult> {
    lease::execute(
        lease,
        Repay::new(info.funds, env),
        &env.contract.address,
        querier,
    )
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
    let IntoDTOResult { lease, batch } = lease::execute(
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
    lease::execute(
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
    lease::execute(
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
