use std::{fmt::Display, result::Result as StdResult};

use platform::{
    ica::ErrorResponse as ICAErrorResponse,
    message::Response as MessageResponse,
    state_machine::{self, Response as StateMachineResponse},
};
use sdk::cosmwasm_std::{Binary, Env, QuerierWrapper, Reply};

use crate::error::{Error, Result as DexResult};

pub type Response<H> = StateMachineResponse<<H as Handler>::Response>;
pub type ContinueResult<H> = DexResult<Response<H>>;
pub enum Result<H>
where
    H: Handler,
{
    Continue(ContinueResult<H>),
    Finished(H::SwapResult),
}

pub fn res_continue<R, S, H>(resp: R, next_state: S) -> ContinueResult<H>
where
    R: Into<MessageResponse>,
    S: Into<H::Response>,
    H: Handler,
{
    Ok(StateMachineResponse::from(resp, next_state))
}

pub fn res_finished<H>(res: H::SwapResult) -> Result<H>
where
    H: Handler,
{
    Result::Finished(res)
}

pub trait Handler
where
    Self: Sized + Display,
{
    type Response;
    type SwapResult;

    fn on_open_ica(
        self,
        _counterparty_version: String,
        _querier: QuerierWrapper<'_>,
        _env: Env,
    ) -> ContinueResult<Self> {
        Err(err(self, "handle open ica response"))
    }

    /// The entry point of a response delivery
    fn on_response(self, _data: Binary, _querier: QuerierWrapper<'_>, _env: Env) -> Result<Self> {
        Err(err(self, "handle transaction response")).into()
    }

    /// The entry point of an error delivery
    fn on_error(
        self,
        response: ICAErrorResponse,
        _querier: QuerierWrapper<'_>,
        _env: Env,
    ) -> Result<Self> {
        Err(err(self, &format!("handle {}", response))).into()
    }

    /// The entry point of a timeout delivery
    fn on_timeout(self, _querier: QuerierWrapper<'_>, _env: Env) -> ContinueResult<Self> {
        Err(err(self, "handle transaction timeout"))
    }

    /// The actual delivery of a response
    ///
    /// Intended to act as a level of indirection allowing a common error handling
    fn on_inner(self, _querier: QuerierWrapper<'_>, _env: Env) -> Result<Self> {
        Err(err(self, "handle inner")).into()
    }

    /// The actual delivery of an ICA open response, error, and timeout
    ///
    /// They are separated from the regular response delivery because they cannot bring the state machine into a final state.
    ///
    /// Intended to act as a level of indirection allowing a common error handling
    fn on_inner_continue(self, _querier: QuerierWrapper<'_>, _env: Env) -> ContinueResult<Self> {
        Err(err(self, "handle inner to 'Continue' response"))
    }

    fn heal(self, _querier: QuerierWrapper<'_>, _env: Env) -> Result<Self> {
        Err(err(self, "handle heal")).into()
    }

    fn reply(self, _querier: QuerierWrapper<'_>, _env: Env, _msg: Reply) -> ContinueResult<Self> {
        Err(err(self, "handle reply"))
    }

    fn on_time_alarm(self, _querier: QuerierWrapper<'_>, _env: Env) -> Result<Self> {
        Err(err(self, "handle time alarm")).into()
    }
}

impl<H> Result<H>
where
    H: Handler,
{
    pub fn map_into<HTo>(self) -> Result<HTo>
    where
        HTo: Handler<SwapResult = H::SwapResult>,
        H::Response: Into<HTo::Response>,
    {
        match self {
            Result::Continue(cont_res) => Result::Continue(cont_res.map(state_machine::from)),
            Result::Finished(finish_res) => Result::Finished(finish_res),
        }
    }
}

impl<H, StateTo, Err> From<Result<H>> for StdResult<StateMachineResponse<StateTo>, Err>
where
    H: Handler<SwapResult = Self>,
    H::Response: Into<StateTo>,
    Error: Into<Err>,
{
    fn from(value: Result<H>) -> Self {
        match value {
            Result::Continue(cont_res) => cont_res.map(state_machine::from).map_err(Into::into),
            Result::Finished(finish_res) => finish_res,
        }
    }
}

pub(crate) fn err<S>(state: S, op: &str) -> Error
where
    S: Display,
{
    Error::unsupported_operation(op, state)
}

impl<H> From<ContinueResult<H>> for Result<H>
where
    H: Handler,
{
    fn from(value: ContinueResult<H>) -> Self {
        Self::Continue(value)
    }
}

impl<H> From<Error> for Result<H>
where
    H: Handler,
{
    fn from(value: Error) -> Self {
        Self::Continue(Err(value))
    }
}
