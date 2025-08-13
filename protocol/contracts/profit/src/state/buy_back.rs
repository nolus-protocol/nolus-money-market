use currency::{CurrencyDef, Group};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};

use access_control::{
    permissions::{ContractOwnerPermission, DexResponseSafeDeliveryPermission},
    user::User,
};
use currencies::{Native, Nls, PaymentGroup};
use dex::{
    AcceptAnyNonZeroSwap, Account, AnomalyTreatment, CheckType, ContractInSwap, Handler,
    Response as DexResponse, Result as DexResult, Stage, StateLocalOut, SwapOutputTask, SwapTask,
    WithCalculator, WithOutputTask,
};
use finance::{
    coin::{Coin, CoinDTO},
    duration::Duration,
};
use oracle::stub::SwapPath;
use platform::bank::{self, BankAccountView};
use sdk::cosmwasm_std::{Addr, Env, QuerierWrapper, Timestamp};
use timealarms::stub::{TimeAlarmDelivery, TimeAlarmsRef};

use crate::{msg::ConfigResponse, result::ContractResult};

use super::{Config, State, StateEnum, SwapClient, idle::Idle, resp_delivery::ForwardToDexEntry};

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

        account
            .balance()
            .map_err(Into::into)
            .and_then(|balance_nls| {
                let next_state = Idle::new(self.config, self.account);

                next_state
                    .send_nls(env, querier, account, balance_nls)
                    .map(|response| {
                        DexResponse::<State>::from(response, State(StateEnum::Idle(next_state)))
                    })
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

impl Handler for BuyBack {
    type Response = State;
    type SwapResult = ContractResult<DexResponse<State>>;

    fn check_permission<U>(
        &self,
        user: &U,
        check_type: CheckType,
        contract_addr: Option<Addr>,
    ) -> DexResult<bool>
    where
        U: User,
    {
        match check_type {
            CheckType::Timealarm => {
                access_control::check(&TimeAlarmDelivery::new(&self.config.time_alarms()), user)?;
            }
            CheckType::ContractOwner => {
                access_control::check(
                    &ContractOwnerPermission::new(&self.config.contract_owner()),
                    &user,
                )?;
            }
            CheckType::DexResponseSafeDelivery => {
                access_control::check(
                    &DexResponseSafeDeliveryPermission::new(&contract_addr),
                    &user,
                )?;
            }
            CheckType::None => {}
        }
    }
}

impl Display for BuyBack {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!("BuyBack"))
    }
}
