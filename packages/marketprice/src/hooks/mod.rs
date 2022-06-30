use cosmwasm_std::{Addr, Binary, StdResult, Timestamp};

pub mod errors;
pub mod price;

pub type Id = u64;

pub trait HookDispatcher {
    fn send_to(
        &mut self,
        id: Id,
        addr: Addr,
        ctime: Timestamp,
        data: &Option<Binary>,
    ) -> StdResult<()>;
}
