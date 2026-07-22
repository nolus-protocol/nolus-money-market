use enum_dispatch::enum_dispatch;

use crate::api::LeasePaymentCurrencies;
use finance::duration::Duration;
use finance::instant::Instant;
use platform::remote::ErrorResponse as ICAErrorResponse;
use remote_lease::callback::RemoteLeaseCallback;
use sdk::cosmwasm_std::{Binary, Env, MessageInfo, QuerierWrapper, Reply};

use crate::{
    api::{
        position::{ClosePolicyChange, PositionClose},
        query::StateResponse,
    },
    error::{ContractError, ContractResult},
};

use super::state::Response;

#[enum_dispatch]
pub(super) trait Contract
where
    Self: Sized,
{
    fn on_dex_response(
        self,
        _response: Binary,
        _querier: QuerierWrapper<'_>,
        _env: Env,
    ) -> ContractResult<Response> {
        err("dex response")
    }

    fn on_dex_error(
        self,
        resp: ICAErrorResponse,
        _querier: QuerierWrapper<'_>,
        _env: Env,
    ) -> ContractResult<Response> {
        err(format!("dex error({resp})",))
    }

    fn on_dex_timeout(self, _querier: QuerierWrapper<'_>, _env: Env) -> ContractResult<Response> {
        err("dex timeout")
    }

    /// The entry point for a remote-lease callback
    ///
    /// Only the states that have initiated remote chain operations should
    /// implement it. The default implementation rejects the callback with
    /// `UnsupportedOperation`;
    /// That same states should enter the existing
    /// `ResponseDelivery` + `DexCallback` safe-delivery mechanism forwarding
    /// the callback through `on_dex_response` / `on_dex_error` / `on_dex_timeout`.
    fn on_remote_lease_callback(
        self,
        _callback: RemoteLeaseCallback<LeasePaymentCurrencies>,
        _info: MessageInfo,
        _querier: QuerierWrapper<'_>,
        _env: Env,
    ) -> ContractResult<Response> {
        err("remote lease callback")
    }

    /// The inner entry point for safe delivery of a Dex response
    ///
    /// The aim is to plug another level in the Cosmwasm messages tree. That allows the code
    /// to handle errors that might occur in the sub-messages, not only in the main one.
    /// Cosmwasm guarantees that it would call `reply` when the sub-message is scheduled
    /// with the correct flag, e.g. ReplyOn::Error or ReplyOn::Always.
    /// Intended to be invoked always by the same contract instance.
    /// The anticipated execution flow, for example when delivering a Dex response, is
    /// `on_dex_response`, `on_dex_inner`, sub-message-1, ... sub-message-N, `reply`
    fn on_dex_inner(self, _querier: QuerierWrapper<'_>, _env: Env) -> ContractResult<Response> {
        err("dex inner")
    }

    fn heal(
        self,
        _querier: QuerierWrapper<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        err("heal")
    }

    fn state(
        self,
        now: Instant,
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
}

fn err<R, Op>(op: Op) -> ContractResult<R>
where
    Op: ToString,
{
    Err(ContractError::unsupported_operation(op))
}
