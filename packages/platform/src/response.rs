use serde::Serialize;

use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{to_binary, Binary, StdResult as CwResult},
};

#[inline]
pub fn response<T>(response: &T) -> CwResult<Response>
where
    T: Serialize + ?Sized,
{
    response_with_messages(response, CwResponse::new())
}

pub fn response_with_messages<T, U>(response: &T, messages: U) -> CwResult<Response>
where
    T: Serialize + ?Sized,
    U: Into<CwResponse>,
{
    let messages: CwResponse = messages.into();

    debug_assert_eq!(messages.data, None, "Overwriting previous response!");

    to_binary(response).map(|binary: Binary| Response(messages.set_data(binary)))
}

#[repr(transparent)]
pub struct Response(CwResponse);

impl From<Response> for CwResponse {
    fn from(value: Response) -> Self {
        value.0
    }
}
