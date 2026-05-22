use serde::{Deserialize, Serialize};

use dex::{Contract as DexContract, Handler as DexHandler};
use finance::duration::Duration;
use finance::instant::Instant;
use platform::{ica::ErrorResponse as ICAErrorResponse, state_machine};
use remote_lease::callback::RemoteLeaseCallback;
use sdk::cosmwasm_std::{self, Binary, Env, MessageInfo, QuerierWrapper, Reply};

use crate::{
    api::query::StateResponse as QueryStateResponse,
    contract::{api::Contract, state::StateResponse as ContractStateResponse},
    error::{ContractError, ContractResult},
};

use super::{Response, State as ContractState};

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct State<H> {
    handler: H,
}

impl<H> State<H> {
    pub fn new(handler: H) -> Self {
        Self { handler }
    }
}

impl<H> Contract for State<H>
where
    H: DexHandler<SwapResult = ContractResult<Response>>,
    H: DexContract<StateResponse = ContractResult<QueryStateResponse>>,
    H::Response: Into<ContractState>,
    Self: Into<ContractState>,
{
    fn on_open_ica(
        self,
        counterparty_version: String,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> ContractResult<Response> {
        self.handler
            .on_open_ica(counterparty_version, querier, env)
            .map(state_machine::from)
            .map_err(Into::into)
    }

    fn on_dex_response(
        self,
        data: Binary,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> ContractResult<Response> {
        self.handler.on_response(data, querier, env).into()
    }

    fn on_dex_error(
        self,
        details: ICAErrorResponse,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> ContractResult<Response> {
        self.handler.on_error(details, querier, env).into()
    }

    fn on_dex_timeout(self, querier: QuerierWrapper<'_>, env: Env) -> ContractResult<Response> {
        self.handler
            .on_timeout(querier, env)
            .map(state_machine::from)
            .map_err(Into::into)
    }

    fn on_remote_lease_callback(
        self,
        callback: RemoteLeaseCallback,
        info: MessageInfo,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> ContractResult<Response> {
        self.handler
            .authz_remote_callback(querier, &info)
            .map_err(ContractError::from)
            .and_then(|()| match callback {
                RemoteLeaseCallback::OperationOk(response) => {
                    cosmwasm_std::to_json_binary(&response)
                        .map_err(Into::into)
                        .and_then(|data| self.on_dex_response(data, querier, env))
                }
                RemoteLeaseCallback::OperationErr(message) => self.on_dex_error(
                    ICAErrorResponse::from(message.as_str().to_owned()),
                    querier,
                    env,
                ),
                RemoteLeaseCallback::OperationTimeout => self.on_dex_timeout(querier, env),
            })
    }

    fn on_dex_inner(self, querier: QuerierWrapper<'_>, env: Env) -> ContractResult<Response> {
        self.handler.on_inner(querier, env).into()
    }

    fn on_dex_inner_continue(
        self,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> ContractResult<Response> {
        self.handler
            .on_inner_continue(querier, env)
            .map(state_machine::from)
            .map_err(Into::into)
    }

    fn heal(
        self,
        querier: QuerierWrapper<'_>,
        env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        self.handler.heal(querier, env).into()
    }

    fn state(
        self,
        now: Instant,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> ContractResult<ContractStateResponse> {
        self.handler.state(now, due_projection, querier)
    }

    fn reply(self, querier: QuerierWrapper<'_>, env: Env, msg: Reply) -> ContractResult<Response> {
        self.handler
            .reply(querier, env, msg)
            .map(state_machine::from)
            .map_err(Into::into)
    }

    fn on_time_alarm(
        self,
        querier: QuerierWrapper<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        self.handler.on_time_alarm(querier, env, info).into()
    }

    fn on_price_alarm(
        self,
        _querier: QuerierWrapper<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        super::ignore_msg(self)
    }
}
