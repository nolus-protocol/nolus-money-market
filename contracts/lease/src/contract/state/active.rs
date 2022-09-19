use std::fmt::Display;

use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, QuerierWrapper};
use platform::{bank::BankStub, batch::Emitter};
use serde::{Deserialize, Serialize};

use crate::{
    contract::{
        alarms::{price::PriceAlarm, time::TimeAlarm, AlarmResult},
        close::Close,
        cmd::LeaseState,
        repay::{Repay, RepayResult},
    },
    error::ContractResult,
    lease::{self, LeaseDTO},
    msg::{ExecuteMsg, StateQuery},
};

use super::{Controller, Response};

#[derive(Serialize, Deserialize)]
pub struct Active {}

impl Controller for Active {
    fn execute(
        self,
        deps: &mut DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> ContractResult<Response> {
        let lease = LeaseDTO::load(deps.storage)?;

        let account = BankStub::my_account(&env, &deps.querier);

        let cw_resp = match msg {
            ExecuteMsg::Repay() => {
                let RepayResult { lease_dto, emitter } =
                    try_repay(&deps.querier, &env, account, info, lease)?;

                lease_dto.store(deps.storage)?;

                Ok(emitter.into())
            }
            ExecuteMsg::Close() => {
                try_close(&deps.querier, &env, account, info, lease).map(Into::into)
            }
            ExecuteMsg::PriceAlarm() => {
                let AlarmResult {
                    response,
                    lease_dto: lease,
                } = try_on_price_alarm(&deps.querier, &env, account, info, lease)?;

                lease.store(deps.storage)?;

                Ok(response)
            }
            ExecuteMsg::TimeAlarm() => {
                let AlarmResult {
                    response,
                    lease_dto: lease,
                } = try_on_time_alarm(&deps.querier, &env, account, info, lease)?;

                lease.store(deps.storage)?;

                Ok(response)
            }
        }?;
        Ok(Response::from(cw_resp, self))
    }

    fn query(self, deps: Deps, env: Env, _msg: StateQuery) -> ContractResult<Binary> {
        let lease = LeaseDTO::load(deps.storage)?;

        let bank = BankStub::my_account(&env, &deps.querier);

        // TODO think on taking benefit from having a LppView trait
        lease::execute(
            lease,
            LeaseState::new(env.block.time, bank),
            &env.contract.address,
            &deps.querier,
        )
    }
}

impl Display for Active {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("active lease")
    }
}

fn try_repay(
    querier: &QuerierWrapper,
    env: &Env,
    account: BankStub,
    info: MessageInfo,
    lease: LeaseDTO,
) -> ContractResult<RepayResult> {
    lease::execute(
        lease,
        Repay::new(&info.funds, account, env),
        &env.contract.address,
        querier,
    )
}

fn try_close(
    querier: &QuerierWrapper,
    env: &Env,
    account: BankStub,
    info: MessageInfo,
    lease: LeaseDTO,
) -> ContractResult<Emitter> {
    let emitter = lease::execute(
        lease,
        Close::new(
            &info.sender,
            env.contract.address.clone(),
            account,
            env.block.time,
        ),
        &env.contract.address,
        querier,
    )?;

    Ok(emitter)
}

fn try_on_price_alarm(
    querier: &QuerierWrapper,
    env: &Env,
    account: BankStub,
    info: MessageInfo,
    lease: LeaseDTO,
) -> ContractResult<AlarmResult> {
    lease::execute(
        lease,
        PriceAlarm::new(env, &info.sender, account, env.block.time),
        &env.contract.address,
        querier,
    )
}

fn try_on_time_alarm(
    querier: &QuerierWrapper,
    env: &Env,
    account: BankStub,
    info: MessageInfo,
    lease: LeaseDTO,
) -> ContractResult<AlarmResult> {
    lease::execute(
        lease,
        TimeAlarm::new(env, &info.sender, account, env.block.time),
        &env.contract.address,
        querier,
    )
}
