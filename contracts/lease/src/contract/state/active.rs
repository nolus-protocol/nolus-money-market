use std::fmt::Display;

use cosmwasm_std::{
    Binary, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, Response as CwResponse,
};
use serde::{Deserialize, Serialize};

use platform::{bank::BankStub, batch::Emitter};

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
pub struct Active {
    pub(super) lease: LeaseDTO,
}

impl Controller for Active {
    fn execute(
        self,
        deps: &mut DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> ContractResult<Response> {
        let account = BankStub::my_account(&env, &deps.querier);

        let resp = match msg {
            ExecuteMsg::Repay() => {
                let RepayResult {
                    lease_dto: lease_updated,
                    emitter,
                } = try_repay(&deps.querier, &env, account, info, self.lease)?;

                into_resp(emitter, lease_updated)
            }
            ExecuteMsg::Close() => {
                let lease_cloned = self.lease.clone();
                let resp = try_close(&deps.querier, &env, account, info, self.lease)?;

                into_resp(resp, lease_cloned)
            }
            ExecuteMsg::PriceAlarm() => {
                let AlarmResult {
                    response,
                    lease_dto: lease_updated,
                } = try_on_price_alarm(&deps.querier, &env, account, info, self.lease)?;

                into_resp(response, lease_updated)
            }
            ExecuteMsg::TimeAlarm() => {
                let AlarmResult {
                    response,
                    lease_dto: lease_updated,
                } = try_on_time_alarm(&deps.querier, &env, account, info, self.lease)?;

                into_resp(response, lease_updated)
            }
        };
        Ok(resp)
    }

    fn query(self, deps: Deps, env: Env, _msg: StateQuery) -> ContractResult<Binary> {
        let bank = BankStub::my_account(&env, &deps.querier);

        // TODO think on taking benefit from having a LppView trait
        lease::execute(
            self.lease,
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

fn into_resp<R>(resp: R, lease: LeaseDTO) -> Response
where
    R: Into<CwResponse>,
{
    Response::from(resp, Active { lease })
}
