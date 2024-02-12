use std::slice::Iter as SliceIter;

use serde::{Deserialize, Serialize};

use currencies::{Native, Nls, PaymentGroup};
use currency::{Currency, SymbolSlice};
use dex::{
    Account, CoinVisitor, Enterable, IterNext, IterState, Response as DexResponse, StateLocalOut,
    SwapTask,
};
use finance::coin::{Coin, CoinDTO};
use oracle_platform::OracleRef;
use platform::{
    bank::{self, BankAccountView},
    message::Response as PlatformResponse,
    never::Never,
};
use sdk::cosmwasm_std::{Addr, Env, QuerierWrapper};
use timealarms::stub::TimeAlarmsRef;

use crate::{error::ContractError, msg::ConfigResponse, profit::Profit, result::ContractResult};

use super::{
    idle::Idle,
    resp_delivery::{ForwardToDexEntry, ForwardToDexEntryContinue},
    Config, ConfigManagement, ProfitCurrencies, State, StateEnum, SwapClient,
};

#[derive(Serialize, Deserialize)]
pub(super) struct BuyBack {
    profit_contract: Addr,
    config: Config,
    account: Account,
    coins: Vec<CoinDTO<PaymentGroup>>,
}

impl BuyBack {
    const QUERY_ERROR: &'static str =
        "Configuration querying is not supported while performing buy back!";

    /// Until [issue #7](https://github.com/nolus-protocol/nolus-money-market/issues/7)
    /// is closed, best action is to verify the pinkie-promise
    /// to not pass in [native currencies](Native) via a debug
    /// assertion.
    pub fn new(
        profit_contract: Addr,
        config: Config,
        account: Account,
        coins: Vec<CoinDTO<PaymentGroup>>,
    ) -> Self {
        debug_assert!(
            coins.iter().all(
                |coin_dto: &CoinDTO<PaymentGroup>| currency::validate::<Native>(coin_dto.ticker())
                    .is_err()
            ),
            "{:?}",
            coins
        );

        Self {
            profit_contract,
            config,
            account,
            coins,
        }
    }
}

impl SwapTask for BuyBack {
    type OutG = Native;
    type Label = String;
    type StateResponse = Never;
    type Result = ContractResult<DexResponse<State>>;

    fn label(&self) -> Self::Label {
        String::from("BuyBack")
    }

    fn dex_account(&self) -> &Account {
        &self.account
    }

    fn oracle(&self) -> &OracleRef {
        self.config.oracle()
    }

    fn time_alarm(&self) -> &TimeAlarmsRef {
        self.config.time_alarms()
    }

    fn out_currency(&self) -> &SymbolSlice {
        Nls::TICKER
    }

    fn on_coins<Visitor>(&self, visitor: &mut Visitor) -> Result<IterState, Visitor::Error>
    where
        Visitor: CoinVisitor<Result = IterNext>,
    {
        let mut coins_iter: SliceIter<'_, CoinDTO<PaymentGroup>> = self.coins.iter();

        TryFind::try_find(&mut coins_iter, |coin: &&CoinDTO<PaymentGroup>| {
            visitor
                .visit(coin)
                .map(|result: IterNext| matches!(result, IterNext::Stop))
        })
        .map(|_| {
            if coins_iter.as_slice().is_empty() {
                IterState::Complete
            } else {
                IterState::Incomplete
            }
        })
    }

    fn finish(
        self,
        _: CoinDTO<Self::OutG>,
        env: &Env,
        querier: QuerierWrapper<'_>,
    ) -> Self::Result {
        let account = bank::account(&self.profit_contract, querier);

        let balance_nls: Coin<Nls> = account.balance()?;

        let bank_response: PlatformResponse =
            Profit::transfer_nls(account, self.config.treasury(), balance_nls, env);

        let next_state: Idle = Idle::new(self.config, self.account);

        Ok(DexResponse::<State> {
            response: next_state
                .enter(env.block.time, querier)
                .map(PlatformResponse::messages_only)
                .map(|state_response: PlatformResponse| state_response.merge_with(bank_response))?,
            next_state: State(StateEnum::Idle(next_state)),
        })
    }
}

impl ConfigManagement
    for StateLocalOut<
        BuyBack,
        ProfitCurrencies,
        SwapClient,
        ForwardToDexEntry,
        ForwardToDexEntryContinue,
    >
{
    fn try_query_config(&self) -> ContractResult<ConfigResponse> {
        Err(ContractError::unsupported_operation(BuyBack::QUERY_ERROR))
    }
}

trait TryFind
where
    Self: Iterator,
{
    fn try_find<F, E>(&mut self, mut f: F) -> Result<Option<Self::Item>, E>
    where
        F: FnMut(&Self::Item) -> Result<bool, E>,
    {
        for item in self {
            if f(&item)? {
                return Ok(Some(item));
            }
        }

        Ok(None)
    }
}

impl<I> TryFind for I where I: Iterator + ?Sized {}

#[cfg(test)]
mod tests {
    use currencies::test::{
        PaymentC3, PaymentC4, PaymentC5, PaymentC6, PaymentC7, PaymentCurrencies, StableC1,
    };
    use dex::{CoinVisitor, IterNext, IterState, SwapTask as _};
    use finance::coin::{Coin, CoinDTO};
    use platform::never::Never;

    use super::BuyBack;

    fn buy_back_instance(coins: Vec<CoinDTO<PaymentCurrencies>>) -> BuyBack {
        use dex::{Account, ConnectionParams, Ics20Channel};
        use oracle_platform::OracleRef;
        use platform::ica::HostAccount;
        use sdk::cosmwasm_std::Addr;
        use timealarms::stub::TimeAlarmsRef;

        use crate::state::Config;

        BuyBack::new(
            Addr::unchecked("DEADCODE"),
            Config::new(
                24,
                Addr::unchecked("DEADCODE"),
                OracleRef::unchecked::<_, StableC1>("DEADCODE"),
                TimeAlarmsRef::unchecked("DEADCODE"),
            ),
            Account::unchecked(
                Addr::unchecked("DEADCODE"),
                HostAccount::try_from(String::from("DEADCODE")).unwrap(),
                ConnectionParams {
                    connection_id: String::from("DEADCODE"),
                    transfer_channel: Ics20Channel {
                        local_endpoint: String::from("DEADCODE"),
                        remote_endpoint: String::from("DEADCODE"),
                    },
                },
            ),
            coins,
        )
    }

    #[repr(transparent)]
    struct Visitor {
        stop_after: Option<usize>,
    }

    impl Visitor {
        fn new(stop_after: Option<usize>) -> Self {
            Self { stop_after }
        }
    }

    impl CoinVisitor for Visitor {
        type Result = IterNext;

        type Error = Never;

        fn visit<G>(&mut self, _: &CoinDTO<G>) -> Result<Self::Result, Self::Error>
        where
            G: currency::Group,
        {
            if let Some(stop_after) = &mut self.stop_after {
                if *stop_after == 0 {
                    return Ok(IterNext::Stop);
                }

                *stop_after -= 1;
            }

            Ok(IterNext::Continue)
        }
    }

    #[test]
    fn always_continue() {
        let buy_back: BuyBack = buy_back_instance(vec![
            Coin::<PaymentC7>::new(100).into(),
            Coin::<PaymentC4>::new(200).into(),
        ]);

        assert_eq!(
            buy_back.on_coins(&mut Visitor::new(None)).unwrap(),
            IterState::Complete
        );
    }

    #[test]
    fn stop_on_first() {
        let buy_back: BuyBack = buy_back_instance(vec![
            Coin::<PaymentC3>::new(100).into(),
            Coin::<StableC1>::new(200).into(),
        ]);

        assert_eq!(
            buy_back.on_coins(&mut Visitor::new(Some(0))).unwrap(),
            IterState::Incomplete
        );
    }

    #[test]
    fn stop_on_second() {
        let buy_back: BuyBack = buy_back_instance(vec![
            Coin::<PaymentC6>::new(100).into(),
            Coin::<PaymentC5>::new(200).into(),
        ]);

        assert_eq!(
            buy_back.on_coins(&mut Visitor::new(Some(1))).unwrap(),
            IterState::Complete
        );
    }
}
