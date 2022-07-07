use cosmwasm_std::{coins, testing::mock_env, Addr, BlockInfo, Coin};
use cw_multi_test::{App, AppBuilder};
use finance::currency::{Currency, Nls};

#[cfg(test)]
#[allow(dead_code)]
pub mod dispatcher_wrapper;
pub mod lease_wrapper;
#[cfg(test)]
pub mod leaser_wrapper;
#[cfg(test)]
#[allow(dead_code)]
pub mod lpp_wrapper;
pub mod oracle_wrapper;
pub mod profit_wrapper;
#[cfg(test)]
pub mod test_case;
pub mod treasury_wrapper;

pub const USER: &str = "user";
pub const ADMIN: &str = "admin";
pub const NATIVE_DENOM: &str = Nls::SYMBOL;

pub fn mock_app(init_funds: &[Coin]) -> App {
    let return_time = mock_env().block.time.minus_seconds(400 * 24 * 60 * 60);

    let mock_start_block = BlockInfo {
        height: 12_345,
        time: return_time,
        chain_id: "cosmos-testnet-14002".to_string(),
    };

    let mut funds = coins(1000, NATIVE_DENOM);
    funds.append(&mut init_funds.to_vec());

    AppBuilder::new()
        .with_block(mock_start_block)
        .build(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &Addr::unchecked(ADMIN), funds)
                .unwrap();
        })
}
