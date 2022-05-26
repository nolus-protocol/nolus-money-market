use cosmwasm_std::{testing::mock_env, Addr, BlockInfo, Coin};
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
    let return_time = mock_env().block.time.minus_seconds(120 * 24 * 60 * 60);

    let mock_start_block = BlockInfo {
        height: 12_345,
        time: return_time,
        chain_id: "cosmos-testnet-14002".to_string(),
    };
    AppBuilder::new()
        .with_block(mock_start_block)
        .build(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &Addr::unchecked(ADMIN), init_funds.to_vec())
                .unwrap();
        })
}
