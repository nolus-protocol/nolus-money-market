use serde::{Deserialize, Serialize};

use currency::{native::Nls, Currency};
use finance::{coin::Coin, duration::Duration, percent::Percent};
use platform::coin_legacy;
use sdk::{
    cosmwasm_ext::CustomMsg,
    cosmwasm_std::{
        testing::mock_env, to_binary, Addr, Binary, BlockInfo, Coin as CwCoin, Deps, Empty, Env,
        StdResult, Timestamp,
    },
    testing::{self, new_app, App, CustomMessageSender},
};

pub(crate) const BASE_INTEREST_RATE: Percent = Percent::from_permille(70);
pub(crate) const UTILIZATION_OPTIMAL: Percent = Percent::from_permille(700);
pub(crate) const ADDON_OPTIMAL_INTEREST_RATE: Percent = Percent::from_permille(20);

type ContractWrapper<
    ExecMsg,
    ExecErr,
    InstMsg,
    InstErr,
    QueryMsg,
    QueryErr,
    Sudo = Empty,
    SudoErr = anyhow::Error,
    ReplyErr = anyhow::Error,
    MigrMsg = Empty,
    MigrErr = anyhow::Error,
> = testing::ContractWrapper<
    ExecMsg,   // execute msg
    InstMsg,   // instantiate msg
    QueryMsg,  // query msg
    ExecErr,   // execute err
    InstErr,   // instantiate err
    QueryErr,  // query err
    CustomMsg, // C
    Empty,     // Q
    Sudo,      // sudo msg
    SudoErr,   // sudo err
    ReplyErr,  // reply err
    MigrMsg,   // migrate msg
    MigrErr,   // migrate err
>;

#[cfg(test)]
pub mod dispatcher_wrapper;
pub mod lease_wrapper;
#[cfg(test)]
pub mod leaser_wrapper;
#[cfg(test)]
pub mod lpp_wrapper;
pub mod oracle_wrapper;
pub mod profit_wrapper;
pub mod timealarms_wrapper;

#[cfg(test)]
pub mod test_case;
pub mod treasury_wrapper;

pub const USER: &str = "user";
pub const ADMIN: &str = "admin";
pub type Native = Nls;

pub fn native_cwcoin<A>(amount: A) -> CwCoin
where
    A: Into<Coin<Native>>,
{
    cwcoin::<Native, A>(amount)
}

pub fn cwcoin<C, A>(amount: A) -> CwCoin
where
    C: Currency,
    A: Into<Coin<C>>,
{
    coin_legacy::to_cosmwasm(amount.into())
}

#[derive(Serialize, Clone, Debug, PartialEq)]
struct MockResponse {}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct MockQueryMsg {}

fn mock_query(_deps: Deps<'_>, _env: Env, _msg: MockQueryMsg) -> StdResult<Binary> {
    to_binary(&MockResponse {})
}

pub(crate) type MockApp = App<CustomMsg, Empty>;

pub(crate) fn mock_app(message_sender: CustomMessageSender, init_funds: &[CwCoin]) -> MockApp {
    let return_time = mock_env().block.time.minus_seconds(400 * 24 * 60 * 60);

    let mock_start_block = BlockInfo {
        height: 12_345,
        time: return_time,
        chain_id: "cosmos-testnet-14002".to_string(),
    };

    let mut funds = vec![native_cwcoin(100000)];
    funds.append(&mut init_funds.to_vec());

    new_app(message_sender)
        .with_block(mock_start_block)
        .build(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &Addr::unchecked(ADMIN), funds)
                .unwrap();
        })
}

pub(crate) trait AppExt {
    fn time_shift(&mut self, t: Duration);
}

impl AppExt for MockApp {
    fn time_shift(&mut self, t: Duration) {
        self.update_block(|block| {
            let ct = block.time.nanos();
            block.time = Timestamp::from_nanos(ct + t.nanos());
            block.height += 1;
        })
    }
}
