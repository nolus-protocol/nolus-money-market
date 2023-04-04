use serde::Serialize;

use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{to_binary, Binary, StdResult},
};

#[inline]
pub fn response<T>(response: &T) -> StdResult<Response>
where
    T: Serialize + ?Sized,
{
    response_with_messages(CwResponse::new(), response)
}

pub fn response_with_messages<T, U>(messages: T, response: &U) -> StdResult<Response>
where
    T: Into<CwResponse>,
    U: Serialize + ?Sized,
{
    to_binary(response).map(|binary: Binary| Response(messages.into().set_data(binary)))
}

#[repr(transparent)]
pub struct Response(CwResponse);

impl From<Response> for CwResponse {
    fn from(value: Response) -> Self {
        value.0
    }
}
