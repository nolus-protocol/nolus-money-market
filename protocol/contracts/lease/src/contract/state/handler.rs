use enum_dispatch::enum_dispatch;

use finance::duration::Duration;
use platform::state_machine::Response as StateMachineResponse;
use sdk::cosmwasm_std::{Env, MessageInfo, QuerierWrapper, Reply, Timestamp};

use crate::{
    api::{
        position::{ClosePolicyChange, PositionClose},
        query::StateResponse,
    },
    error::{ContractError, ContractResult},
};

use super::State;

pub(crate) type Response = StateMachineResponse<State>;

/// The Lease State Machine API
///
/// Most of the methods provide a default, `ContractError::UnsupportedOperation`, implementation
/// since only a subset of the operations are supported by each Lease state.
#[enum_dispatch]
pub(crate) trait Handler
where
    Self: Sized,
{
    fn state(
        self,
        now: Timestamp,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> ContractResult<StateResponse>;

    fn reply(
        self,
        _querier: QuerierWrapper<'_>,
        _env: Env,
        _msg: Reply,
    ) -> ContractResult<Response> {
        err("reply")
    }

    fn repay(
        self,
        _querier: QuerierWrapper<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        err("repay")
    }

    fn change_close_policy(
        self,
        _change: ClosePolicyChange,
        _querier: QuerierWrapper<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        err("change close policy")
    }

    fn close_position(
        self,
        _spec: PositionClose,
        _querier: QuerierWrapper<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        err("close position")
    }

    fn on_time_alarm(
        self,
        _querier: QuerierWrapper<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        err("on time alarm")
    }

    fn on_price_alarm(
        self,
        _querier: QuerierWrapper<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        err("on price alarm")
    }

    fn heal(
        self,
        _querier: QuerierWrapper<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        err("heal")
    }
}

fn err<R>(op: &str) -> ContractResult<R> {
    Err(ContractError::unsupported_operation(op))
}
