use serde::{Deserialize, Serialize};

use ::lease::api::LpnCoinDTO;
use currencies::{Lpn, Nls, PaymentGroup};
use currency::{CurrencyDTO, CurrencyDef, Group, GroupFilterMap, MemberOf, PairsGroup};
use finance::{
    coin::{Amount, Coin},
    duration::Duration,
    percent::Percent100,
};
use platform::coin_legacy;
pub use sdk::cosmwasm_std::Coin as CwCoin;
use sdk::{
    cosmwasm_ext::InterChainMsg,
    cosmwasm_std::{
        Addr, Binary, BlockInfo, Deps, Empty, Env, QuerierWrapper, StdResult,
        StdResult as CwResult, Timestamp, testing::mock_env, to_json_binary,
    },
    testing::{self, CwApp, InterChainMsgSender, new_app},
};

pub(crate) const BASE_INTEREST_RATE: Percent100 = Percent100::from_permille(70);
pub(crate) const UTILIZATION_OPTIMAL: Percent100 = Percent100::from_permille(700);
pub(crate) const ADDON_OPTIMAL_INTEREST_RATE: Percent100 = Percent100::from_permille(20);

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

pub mod ibc;
pub mod lease;
pub mod leaser;
pub mod lpp;
pub mod oracle;
pub mod profit;
pub mod protocols;
pub mod reserve;
pub mod swap;
pub mod test_case;
pub mod timealarms;
pub mod treasury;

pub const USER: &str = "user";
pub const ADMIN: &str = "admin";
pub const LEASE_ADMIN: &str = "lease_admin";

pub fn query_all_balances(addr: &Addr, querier: QuerierWrapper<'_>) -> Vec<CwCoin> {
    #[derive(Clone, Copy)]
    struct QueryAllBalances<'addr, 'querier>(&'addr Addr, QuerierWrapper<'querier>);

    impl GroupFilterMap for QueryAllBalances<'_, '_> {
        type VisitedG = PaymentGroup;

        type Outcome = CwResult<[CwCoin; 2]>;

        fn on<C>(&self, _: &CurrencyDTO<C::Group>) -> Option<Self::Outcome>
        where
            C: CurrencyDef + PairsGroup<CommonGroup = <Self::VisitedG as Group>::TopG>,
            C::Group: MemberOf<Self::VisitedG> + MemberOf<<Self::VisitedG as Group>::TopG>,
        {
            let query = |denom| self.1.query_balance(self.0.clone(), denom);

            Some(
                query(C::bank())
                    .and_then(|bank_coin| query(C::dex()).map(|dex_coin| [bank_coin, dex_coin])),
            )
        }
    }

    let mut balances = PaymentGroup::filter_map(QueryAllBalances(addr, querier))
        .collect::<Result<Vec<_>, _>>()
        .expect("All currencies should be queriable!")
        .into_flattened();

    balances.retain(|coin| !coin.amount.is_zero());

    balances
}

pub fn native_cwcoin(amount: Amount) -> CwCoin {
    cwcoin_from_amount::<Nls>(amount)
}

pub fn lpn_coin(amount: Amount) -> LpnCoinDTO {
    Coin::<Lpn>::new(amount).into()
}

pub fn coin<C>(amount: Amount) -> Coin<C> {
    Coin::<C>::new(amount)
}

pub fn cwcoin<C>(coin: Coin<C>) -> CwCoin
where
    C: CurrencyDef,
{
    coin_legacy::to_cosmwasm_on_nolus(coin)
}

pub fn cwcoin_from_amount<C>(amount: Amount) -> CwCoin
where
    C: CurrencyDef,
{
    cwcoin(coin::<C>(amount))
}

pub fn cwcoin_as_balance<C>(coin: Coin<C>) -> Vec<CwCoin>
where
    C: CurrencyDef,
{
    if coin.is_zero() {
        vec![]
    } else {
        vec![cwcoin::<C>(coin)]
    }
}

pub fn cwcoin_dex<C>(amount: Amount) -> CwCoin
where
    C: CurrencyDef,
{
    coin_legacy::to_cosmwasm_on_dex(coin::<C>(amount))
}

#[derive(Serialize, Clone, Debug, PartialEq)]
struct MockResponse {}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct MockQueryMsg {}

fn dummy_query(_deps: Deps<'_>, _env: Env, _msg: MockQueryMsg) -> StdResult<Binary> {
    to_json_binary(&MockResponse {})
}

pub(crate) type MockApp = CwApp<InterChainMsg, Empty>;

pub(crate) fn mock_app(message_sender: InterChainMsgSender, init_funds: &[CwCoin]) -> MockApp {
    let return_time = mock_env().block.time.minus_seconds(400 * 24 * 60 * 60);

    let mock_start_block = BlockInfo {
        height: 12_345,
        time: return_time,
        chain_id: "nolus-testnet-14002".to_string(),
    };

    let mut funds = vec![native_cwcoin(100000)];
    funds.append(&mut init_funds.to_vec());

    new_app(message_sender)
        .with_block(mock_start_block)
        .build(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &testing::user(ADMIN), funds)
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
