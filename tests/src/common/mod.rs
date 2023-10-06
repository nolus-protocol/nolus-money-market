use ::lease::api::LpnCoin;
use serde::{Deserialize, Serialize};

use currency::{lpn::Usdc, native::Nls, Currency};
use finance::{
    coin::{Amount, Coin},
    duration::Duration,
    percent::Percent,
};
use platform::coin_legacy;
use sdk::{
    cosmwasm_ext::InterChainMsg,
    cosmwasm_std::{
        testing::mock_env, to_binary, Addr, Binary, BlockInfo, Coin as CwCoin, Deps, Empty, Env,
        StdResult, Timestamp,
    },
    testing::{self, new_app, CwApp, InterChainMsgSender},
};

pub(crate) const BASE_INTEREST_RATE: Percent = Percent::from_permille(70);
pub(crate) const UTILIZATION_OPTIMAL: Percent = Percent::from_permille(700);
pub(crate) const ADDON_OPTIMAL_INTEREST_RATE: Percent = Percent::from_permille(20);

type CwContractWrapper<
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
> = testing::CwContractWrapper<
    ExecMsg,       // execute msg
    InstMsg,       // instantiate msg
    QueryMsg,      // query msg
    ExecErr,       // execute err
    InstErr,       // instantiate err
    QueryErr,      // query err
    InterChainMsg, // C
    Empty,         // Q
    Sudo,          // sudo msg
    SudoErr,       // sudo err
    ReplyErr,      // reply err
    MigrMsg,       // migrate msg
    MigrErr,       // migrate err
>;

pub mod dispatcher;
pub mod ibc;
pub mod lease;
pub mod leaser;
pub mod lpp;
pub mod oracle;
pub mod profit;
pub mod swap;
pub mod test_case;
pub mod timealarms;
pub mod treasury;

pub const USER: &str = "user";
pub const ADMIN: &str = "admin";
pub type Native = Nls;
pub type Lpn = Usdc;

pub fn native_cwcoin<A>(amount: A) -> CwCoin
where
    A: Into<Coin<Native>>,
{
    cwcoin::<Native, A>(amount)
}

pub fn lpn_coin(amount: Amount) -> LpnCoin {
    Coin::<Lpn>::new(amount).into()
}

pub fn cwcoin<C, A>(amount: A) -> CwCoin
where
    C: Currency,
    A: Into<Coin<C>>,
{
    coin_legacy::to_cosmwasm(amount.into())
}
pub fn cwcoin_as_balance<C, A>(amount: A) -> Vec<CwCoin>
where
    C: Currency,
    A: Into<Coin<C>> + Copy,
{
    if amount.into().is_zero() {
        vec![]
    } else {
        vec![cwcoin(amount)]
    }
}

#[derive(Serialize, Clone, Debug, PartialEq)]
struct MockResponse {}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct MockQueryMsg {}

fn mock_query(_deps: Deps<'_>, _env: Env, _msg: MockQueryMsg) -> StdResult<Binary> {
    to_binary(&MockResponse {})
}

pub(crate) type MockApp = CwApp<InterChainMsg, Empty>;

pub(crate) fn mock_app(message_sender: InterChainMsgSender, init_funds: &[CwCoin]) -> MockApp {
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
