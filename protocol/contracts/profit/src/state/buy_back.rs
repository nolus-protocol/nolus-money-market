use currency::{CurrencyDef, Group};
use serde::{Deserialize, Serialize};

use currencies::{Native, Nls, PaymentGroup};
use dex::{
    AcceptAnyNonZeroSwap, Account, AnomalyTreatment, ContractInSwap, Enterable,
    Response as DexResponse, Stage, StateLocalOut, SwapOutputTask, SwapTask, WithCalculator,
    WithOutputTask,
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
    Config, ConfigManagement, State, StateEnum, SwapClient, idle::Idle,
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
                .all(|not_native: &CoinDTO<PaymentGroup>| not_native
                    .of_currency_dto(Nls::dto())
                    .is_err()),
            "{coins:?}",
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
    type Label = String;
    type StateResponse = ConfigResponse;
    type Result = ContractResult<DexResponse<State>>;

    fn label(&self) -> Self::Label {
        String::from("BuyBack")
    }

    fn dex_account(&self) -> &Account {
        &self.account
    }

    fn oracle(&self) -> &impl SwapPath<<Self::InG as Group>::TopG> {
        self.config.oracle()
    }

    fn time_alarm(&self) -> &TimeAlarmsRef {
        self.config.time_alarms()
    }

    fn coins(&self) -> impl IntoIterator<Item = CoinDTO<Self::InG>> {
        self.coins.clone().into_iter()
    }

    fn with_slippage_calc<WithCalc>(&self, with_calc: WithCalc) -> WithCalc::Output
    where
        WithCalc: WithCalculator<Self>,
    {
        with_calc.on(&AcceptAnyNonZeroSwap::<
            _,
            <Self as SwapOutputTask<Self>>::OutC,
        >::default())
    }

    fn into_output_task<Cmd>(self, cmd: Cmd) -> Cmd::Output
    where
        Cmd: WithOutputTask<Self>,
    {
        cmd.on(self)
    }
}

impl SwapOutputTask<Self> for BuyBack {
    type OutC = Nls;

    fn as_spec(&self) -> &Self {
        self
    }

    fn into_spec(self) -> Self {
        self
    }

    fn on_anomaly(self) -> AnomalyTreatment<Self> {
        AnomalyTreatment::Retry(self)
    }

    fn finish(
        self,
        _amount_out: Coin<Self::OutC>,
        env: &Env,
        querier: QuerierWrapper<'_>,
    ) -> <Self as SwapTask>::Result {
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

impl ConfigManagement for StateLocalOut<BuyBack, SwapClient, ForwardToDexEntry> {}
