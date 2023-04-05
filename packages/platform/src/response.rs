use serde::Serialize;

use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{to_binary, Binary, StdResult as CwResult},
};

pub fn response<T>(response: &T) -> CwResult<Response>
where
    T: Serialize + ?Sized,
{
    response_with_messages_unchecked(response, CwResponse::new())
}

pub fn response_with_messages<T, U>(response: &T, messages: U) -> CwResult<Response>
where
    T: Serialize + ?Sized,
    U: Into<CwResponse> + 'static,
{
    debug_assert_ne!(
        std::any::TypeId::of::<U>(),
        std::any::TypeId::of::<CwResponse>(),
        "Possible overwriting of previous response!"
    );
    debug_assert_ne!(
        std::any::TypeId::of::<U>(),
        std::any::TypeId::of::<Response>(),
        "Overwriting of previous response!"
    );

    response_with_messages_unchecked(response, messages)
}

#[inline]
fn response_with_messages_unchecked<T, U>(response: &T, messages: U) -> CwResult<Response>
where
    T: Serialize + ?Sized,
    U: Into<CwResponse>,
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
