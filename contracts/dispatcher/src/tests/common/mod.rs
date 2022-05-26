use cosmwasm_std::{Addr, Coin};
use cw_multi_test::{App, AppBuilder};

#[cfg(feature = "cosmwasm")]
#[cfg(test)]
#[allow(dead_code)]
pub mod mock_dispatcher;
pub mod mock_lease;
#[cfg(feature = "cosmwasm")]
#[cfg(test)]
#[allow(dead_code)]
pub mod mock_lpp;
pub mod mock_oracle;
pub mod mock_treasury;

pub const USER: &str = "user";
pub const ADMIN: &str = "admin";

pub fn mock_app(init_funds: &[Coin]) -> App {
    AppBuilder::new().build(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &Addr::unchecked(ADMIN), init_funds.to_vec())
            .unwrap();
    })
}
