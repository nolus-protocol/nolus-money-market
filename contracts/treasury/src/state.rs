use sdk::{
    cosmwasm_std::{Addr, Storage},
    cw_storage_plus::Item,
};

use crate::ContractError;

pub const ADMIN: Item<Addr> = Item::new("admin");
pub const REWARDS_DISPATCHER: Item<Addr> = Item::new("rewards_dispatcher");

pub fn assert_admin(storage: &dyn Storage, addr: Addr) -> Result<(), ContractError> {
    let admin = ADMIN.load(storage)?;
    if addr == admin {
        Ok(())
    } else {
        Err(ContractError::Unauthorized {})
    }
}

pub fn assert_rewards_dispatcher(storage: &dyn Storage, addr: &Addr) -> Result<(), ContractError> {
    let maybe_dispatcher = REWARDS_DISPATCHER.may_load(storage)?;
    let dispatcher = maybe_dispatcher.ok_or(ContractError::NotConfigured {})?;
    if addr.eq(&dispatcher) {
        Ok(())
    } else {
        Err(ContractError::Unauthorized {})
    }
}
