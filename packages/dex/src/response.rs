use std::{fmt::Display, result::Result as StdResult};

use platform::{
    message::Response as MessageResponse,
    state_machine::{self, Response as StateMachineResponse},
};
use sdk::cosmwasm_std::{Api, Binary, Deps, Env};

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
        deps: Deps<'_>,
        _env: Env,
    ) -> ContinueResult<Self> {
        Err(err(self, "handle open ica response", deps.api))
    }

    fn on_response(self, _data: Binary, deps: Deps<'_>, _env: Env) -> Result<Self> {
        Err(err(self, "handle transaction response", deps.api)).into()
    }

    fn on_error(self, deps: Deps<'_>, _env: Env) -> ContinueResult<Self> {
        Err(err(self, "handle transaction error", deps.api))
    }

    fn on_timeout(self, deps: Deps<'_>, _env: Env) -> ContinueResult<Self> {
        Err(err(self, "handle transaction timeout", deps.api))
    }

    fn on_time_alarm(self, deps: Deps<'_>, _env: Env) -> Result<Self> {
        Err(err(self, "handle time alarm", deps.api)).into()
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

fn err<S>(state: S, op: &str, api: &dyn Api) -> Error
where
    S: Display,
{
    let err = Error::unsupported_operation(format!("{op} on {state}"));
    api.debug(&format!("{err}"));
    err
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
