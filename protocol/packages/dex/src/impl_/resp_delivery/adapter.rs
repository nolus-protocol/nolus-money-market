use sdk::cosmwasm_std::{Binary, Deps, Env};

use crate::impl_::{
    response::{self, ContinueResult as ResponseContinueResult, Result as ResponseResult},
    Handler,
};

pub trait DeliveryAdapter<H, Response>
where
    H: Handler,
{
    fn deliver(handler: H, _response: Response, deps: Deps<'_>, _env: Env) -> ResponseResult<H> {
        Err(response::err(
            handler,
            "deliver transaction response",
            deps.api,
        ))
        .into()
    }

    fn deliver_continue(
        handler: H,
        _response: Response,
        deps: Deps<'_>,
        _env: Env,
    ) -> ResponseContinueResult<H> {
        Err(response::err(
            handler,
            "deliver ica_open response, error or timeout",
            deps.api,
        ))
    }
}

pub struct ResponseDeliveryAdapter();
impl<H> DeliveryAdapter<H, Binary> for ResponseDeliveryAdapter
where
    H: Handler,
{
    fn deliver(handler: H, response: Binary, deps: Deps<'_>, env: Env) -> ResponseResult<H> {
        handler.on_response(response, deps, env)
    }
}

pub struct ICAOpenDeliveryAdapter();
impl<H> DeliveryAdapter<H, String> for ICAOpenDeliveryAdapter
where
    H: Handler,
{
    fn deliver_continue(
        handler: H,
        counterparty_version: String,
        deps: Deps<'_>,
        env: Env,
    ) -> ResponseContinueResult<H> {
        handler.on_open_ica(counterparty_version, deps, env)
    }
}
