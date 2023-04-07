use serde::Serialize;

use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{to_binary, Binary},
};

use crate::error::Result;

#[inline]
pub fn response<T>(response: &T) -> Result<Response>
where
    T: Serialize + ?Sized,
{
    response_with_messages(response, CwResponse::new())
}

pub fn response_with_messages<T, U>(response: &T, messages: U) -> Result<Response>
where
    T: Serialize + ?Sized,
    U: Into<CwResponse>,
{
    let messages: CwResponse = messages.into();

    debug_assert_eq!(messages.data, None, "Overwriting previous response!");

    to_binary(response)
        .map_err(Into::into)
        .map(|binary: Binary| Response(messages.set_data(binary)))
}

#[repr(transparent)]
pub struct Response(CwResponse);

impl From<Response> for CwResponse {
    fn from(value: Response) -> Self {
        value.0
    }
}

pub struct StateMachineResponse<State> {
    pub cw_response: CwResponse,
    pub next_state: State,
}

impl<State> StateMachineResponse<State> {
    pub fn from<R, S>(resp: R, next_state: S) -> Self
    where
        R: Into<CwResponse>,
        S: Into<State>,
    {
        Self {
            cw_response: resp.into(),
            next_state: next_state.into(),
        }
    }
}

pub fn from<StateFrom, StateTo>(
    value: StateMachineResponse<StateFrom>,
) -> StateMachineResponse<StateTo>
where
    StateFrom: Into<StateTo>,
{
    let contract_state: StateTo = value.next_state.into();
    StateMachineResponse::from(value.cw_response, contract_state)
}
