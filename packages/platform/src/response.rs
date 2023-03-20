use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{to_binary, StdResult},
};
use serde::Serialize;

pub fn response<T>(resp: &T) -> StdResult<Response>
where
    T: Serialize + ?Sized,
{
    Ok(Response::new().set_data(to_binary(resp)?))
}
