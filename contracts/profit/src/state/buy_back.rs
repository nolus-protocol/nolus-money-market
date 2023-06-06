use serde::{Deserialize, Serialize};

use currency::{
    native::{Native, Nls},
    non_native_payment::NonNativePaymentGroup,
};
use dex::{
    Account, CoinVisitor, Enterable, IterNext, IterState, Response as DexResponse, StateLocalOut,
    SwapTask,
};
use finance::{
    coin::{Coin, CoinDTO},
    currency::{Currency, Symbol},
};
use oracle::stub::OracleRef;
use platform::{
    bank::{self, BankAccountView},
    message::Response as PlatformResponse,
    never::Never,
};
use sdk::cosmwasm_std::{Addr, Env, QuerierWrapper};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    error::ContractError, msg::ConfigResponse, profit::Profit, result::ContractResult,
    typedefs::CadenceHours,
};

use super::{idle::Idle, Config, ConfigManagement, SetupDexHandler, State, StateEnum};

pub type BuyBackCurrencies = NonNativePaymentGroup;

#[derive(Serialize, Deserialize)]
pub(super) struct BuyBack {
    profit_contract: Addr,
    config: Config,
    account: Account,
    coins: Vec<CoinDTO<BuyBackCurrencies>>,
}

impl BuyBack {
    pub fn new(
        profit_contract: Addr,
        config: Config,
        account: Account,
        coins: Vec<CoinDTO<BuyBackCurrencies>>,
    ) -> Self {
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

    fn out_currency(&self) -> Symbol<'_> {
        Nls::TICKER
    }

    fn on_coins<Visitor>(&self, visitor: &mut Visitor) -> Result<IterState, Visitor::Error>
    where
        Visitor: CoinVisitor<Result = IterNext>,
    {
        TryFind::try_find(
            &mut self.coins.iter(),
            |coin: &&CoinDTO<BuyBackCurrencies>| {
                visitor
                    .visit(coin)
                    .map(|result: IterNext| matches!(result, IterNext::Stop))
            },
        )
        .map(|maybe_coin: Option<&CoinDTO<BuyBackCurrencies>>| {
            if maybe_coin.is_some() {
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
        querier: &QuerierWrapper<'_>,
    ) -> Self::Result {
        let account = bank::account(&self.profit_contract, querier);

        let balance_nls: Coin<Nls> = account.balance()?;

        let bank_response: PlatformResponse =
            Profit::transfer_nls(account, env, self.config.treasury(), balance_nls);

        let state: Idle = Idle::new(self.config, self.account);

        Ok(DexResponse::<State> {
            response: state
                .enter(env.block.time, querier)
                .map(PlatformResponse::messages_only)
                .map(|state_response: PlatformResponse| state_response.merge_with(bank_response))?,
            next_state: State(StateEnum::Idle(state)),
        })
    }
}

impl ConfigManagement for StateLocalOut<BuyBack> {
    fn try_update_config(self, _: CadenceHours) -> ContractResult<Self> {
        Err(ContractError::UnsupportedOperation(String::from(
            "Configuration changes are not allowed during ICA opening process.",
        )))
    }

    fn try_query_config(&self) -> ContractResult<ConfigResponse> {
        Err(ContractError::UnsupportedOperation(String::from(
            "Querying configuration is not allowed during buy-back.",
        )))
    }
}

impl SetupDexHandler for StateLocalOut<BuyBack> {
    type State = Self;
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
