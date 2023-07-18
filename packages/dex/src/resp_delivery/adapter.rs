use sdk::cosmwasm_std::{Binary, Deps, Env};

use crate::{response::Result, Handler};

pub trait DeliveryAdapter<H, R>
where
    H: Handler,
{
    fn deliver(handler: H, response: R, deps: Deps<'_>, env: Env) -> Result<H>;
}

pub struct ResponseDeliveryAdapter();
impl<H> DeliveryAdapter<H, Binary> for ResponseDeliveryAdapter
where
    H: Handler,
{
    fn deliver(handler: H, response: Binary, deps: Deps<'_>, env: Env) -> Result<H> {
        handler.on_response(response, deps, env)
    }
}
