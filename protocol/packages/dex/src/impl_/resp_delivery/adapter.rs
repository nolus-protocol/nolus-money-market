use sdk::cosmwasm_std::{Binary, Env, QuerierWrapper};

use crate::impl_::{
    response::{self, ContinueResult as ResponseContinueResult, Result as ResponseResult},
    Handler,
};

pub trait DeliveryAdapter<H, Response>
where
    H: Handler,
{
    fn deliver(
        handler: H,
        _response: Response,
        _querier: QuerierWrapper<'_>,
        _env: Env,
    ) -> ResponseResult<H> {
        Err(response::err(handler, "deliver transaction response")).into()
    }

    fn deliver_continue(
        handler: H,
        _response: Response,
        _querier: QuerierWrapper<'_>,
        _env: Env,
    ) -> ResponseContinueResult<H> {
        Err(response::err(handler, "deliver ica_open response"))
    }

    /// Retry a previously failed response delivery
    ///
    /// Note that it is common for both scenarious - final and non-final DEX state.
    fn deliver_again(
        handler: H,
        _response: Response,
        _querier: QuerierWrapper<'_>,
        _env: Env,
    ) -> ResponseResult<H>;
}

pub struct ResponseDeliveryAdapter();
impl<H> DeliveryAdapter<H, Binary> for ResponseDeliveryAdapter
where
    H: Handler,
{
    fn deliver(
        handler: H,
        response: Binary,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> ResponseResult<H> {
        handler.on_response(response, querier, env)
    }

    fn deliver_again(
        handler: H,
        response: Binary,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> ResponseResult<H> {
        Self::deliver(handler, response, querier, env)
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
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> ResponseContinueResult<H> {
        handler.on_open_ica(counterparty_version, querier, env)
    }

    fn deliver_again(
        handler: H,
        response: String,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> ResponseResult<H> {
        Self::deliver_continue(handler, response, querier, env).into()
    }
}
