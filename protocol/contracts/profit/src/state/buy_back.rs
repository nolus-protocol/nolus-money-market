use serde::{Deserialize, Serialize};

use currencies::{Native, Nls, PaymentGroup};
use currency::CurrencyDTO;
use dex::{
    AcceptAnyNonZeroSwap, Account, AnomalyMonitoredTask, AnomalyPolicy, ContractInSwap, Enterable,
    Response as DexResponse, Stage, StateLocalOut, SwapTask,
};
use finance::{
    coin::{Coin, CoinDTO},
    duration::Duration,
};
use oracle::stub::SwapPath;
use platform::{
    bank::{self, BankAccountView},
    message::Response as PlatformResponse,
};
use sdk::cosmwasm_std::{Addr, Env, QuerierWrapper, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{msg::ConfigResponse, profit::Profit, result::ContractResult};

use super::{
    Config, ConfigManagement, ProfitCurrencies, State, StateEnum, SwapClient, idle::Idle,
    resp_delivery::ForwardToDexEntry,
};

#[derive(Serialize, Deserialize)]
pub(super) struct BuyBack {
    profit_contract: Addr,
    config: Config,
    account: Account,
    coins: Vec<CoinDTO<PaymentGroup>>,
}

impl BuyBack {
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
            coins
                .iter()
                .all(|not_native: &CoinDTO<PaymentGroup>| not_native.currency()
                    != currency::dto::<Nls, PaymentGroup>()),
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
    type InG = PaymentGroup;
    type OutG = Native;
    type InOutG = PaymentGroup;
    type Label = String;
    type StateResponse = ConfigResponse;
    type Result = ContractResult<DexResponse<State>>;

    fn label(&self) -> Self::Label {
        String::from("BuyBack")
    }

    fn dex_account(&self) -> &Account {
        &self.account
    }

    fn oracle(&self) -> &impl SwapPath<Self::InOutG> {
        self.config.oracle()
    }

    fn time_alarm(&self) -> &TimeAlarmsRef {
        self.config.time_alarms()
    }

    fn out_currency(&self) -> CurrencyDTO<Self::OutG> {
        currency::dto::<Nls, Self::OutG>()
    }

    fn coins(&self) -> impl IntoIterator<Item = CoinDTO<Self::InG>> {
        self.coins.clone().into_iter()
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
            Profit::transfer_nls(account, self.config.treasury().clone(), balance_nls, env);

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

impl AnomalyMonitoredTask for BuyBack {
    fn policy(&self) -> impl AnomalyPolicy<Self> {
        AcceptAnyNonZeroSwap::on_task(self)
    }
}

impl ContractInSwap for BuyBack {
    type StateResponse = <Self as SwapTask>::StateResponse;

    fn state(
        self,
        _in_progress: Stage,
        _now: Timestamp,
        _due_projection: Duration,
        _querier: QuerierWrapper<'_>,
    ) -> <Self as SwapTask>::StateResponse {
        ConfigResponse {
            cadence_hours: self.config.cadence_hours(),
        }
    }
}

impl ConfigManagement for StateLocalOut<BuyBack, ProfitCurrencies, SwapClient, ForwardToDexEntry> {}
