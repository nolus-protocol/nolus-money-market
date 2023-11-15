use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    marker::PhantomData,
};

use serde::{Deserialize, Serialize};

use currencies::{Nls, PaymentGroup};
use currency::{Currency, Group};
use dex::{
    Account, Enterable, Error as DexError, Handler, Response as DexResponse, Result as DexResult,
    StartLocalLocalState,
};
use finance::{
    coin::{Coin, CoinDTO, WithCoin, WithCoinResult},
    duration::Duration,
};
use platform::{
    bank::{self, Aggregate, BankAccount, BankAccountView, BankStub, BankView},
    batch::Batch,
    message::Response as PlatformResponse,
    state_machine::Response as StateMachineResponse,
};
use sdk::cosmwasm_std::{Addr, Deps, Env, QuerierWrapper, Timestamp};
use timealarms::result::ContractResult as TimeAlarmsResult;

use crate::{
    error::ContractError, msg::ConfigResponse, profit::Profit, result::ContractResult,
    typedefs::CadenceHours,
};

use super::{
    buy_back::BuyBack,
    resp_delivery::{ForwardToDexEntry, ForwardToDexEntryContinue},
    Config, ConfigManagement, SetupDexHandler, State, StateEnum,
};

#[derive(Serialize, Deserialize)]
pub(super) struct Idle {
    config: Config,
    account: Account,
}

impl Idle {
    pub fn new(config: Config, account: Account) -> Self {
        Self { config, account }
    }

    fn send_nls<B>(
        &self,
        env: &Env,
        querier: QuerierWrapper<'_>,
        account: B,
        nls: Coin<Nls>,
    ) -> ContractResult<PlatformResponse>
    where
        B: BankAccount,
    {
        self.enter(env.block.time, querier)
            .map(PlatformResponse::messages_only)
            .map(|state_response: PlatformResponse| {
                Profit::transfer_nls(account, self.config.treasury(), nls, env)
                    .merge_with(state_response)
            })
            .map_err(Into::into)
    }

    fn on_time_alarm(
        self,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> ContractResult<DexResponse<Self>> {
        let account: BankStub<BankView<'_>> = bank::account(&env.contract.address, querier);

        let balances: SplitCoins<Nls, PaymentGroup> = account
            .balances::<PaymentGroup, _>(CoinToDTO(PhantomData, PhantomData))?
            .transpose()?
            .unwrap_or_default();

        if balances.rest.is_empty() {
            self.send_nls(&env, querier, account, balances.filtered)
                .map(|response: PlatformResponse| DexResponse::<Self> {
                    response,
                    next_state: State(StateEnum::Idle(self)),
                })
        } else {
            self.try_enter_buy_back(querier, env.contract.address, env.block.time, balances.rest)
        }
    }

    fn try_enter_buy_back(
        self,
        querier: QuerierWrapper<'_>,
        profit_addr: Addr,
        now: Timestamp,
        balances: Vec<CoinDTO<PaymentGroup>>,
    ) -> ContractResult<DexResponse<Self>> {
        let state: StartLocalLocalState<BuyBack, ForwardToDexEntry, ForwardToDexEntryContinue> =
            dex::start_local_local(BuyBack::new(
                profit_addr,
                self.config,
                self.account,
                balances,
            ));

        state
            .enter(now, querier)
            .map(|batch: Batch| DexResponse::<Self> {
                response: PlatformResponse::messages_only(batch),
                next_state: State(StateEnum::BuyBack(state.into())),
            })
            .map_err(Into::into)
    }

    fn setup_time_alarm(config: &Config, now: Timestamp) -> TimeAlarmsResult<Batch> {
        config
            .time_alarms()
            .setup_alarm(now + Duration::from_hours(config.cadence_hours()))
    }
}

impl Enterable for Idle {
    fn enter(&self, now: Timestamp, _: QuerierWrapper<'_>) -> Result<Batch, DexError> {
        Self::setup_time_alarm(&self.config, now).map_err(DexError::TimeAlarmError)
    }
}

impl ConfigManagement for Idle {
    fn try_update_config(
        self,
        now: Timestamp,
        cadence_hours: CadenceHours,
    ) -> ContractResult<StateMachineResponse<Self>> {
        let config: Config = self.config.update(cadence_hours);

        Self::setup_time_alarm(&config, now)
            .map(PlatformResponse::messages_only)
            .map(|response: PlatformResponse| StateMachineResponse {
                response,
                next_state: Self { config, ..self },
            })
            .map_err(Into::into)
    }

    fn try_query_config(&self) -> ContractResult<ConfigResponse> {
        Ok(ConfigResponse {
            cadence_hours: self.config.cadence_hours(),
        })
    }
}

impl Handler for Idle {
    type Response = State;
    type SwapResult = ContractResult<DexResponse<State>>;

    fn on_time_alarm(self, deps: Deps<'_>, env: Env) -> DexResult<Self> {
        DexResult::Finished(self.on_time_alarm(deps.querier, env))
    }
}

impl SetupDexHandler for Idle {
    type State = Self;
}

impl Display for Idle {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!("Idle"))
    }
}

struct CoinToDTO<FilterC, G>(PhantomData<FilterC>, PhantomData<G>)
where
    FilterC: Currency,
    G: Group;

impl<FilterC, G> WithCoin for CoinToDTO<FilterC, G>
where
    FilterC: Currency,
    G: Group,
{
    type Output = SplitCoins<FilterC, G>;
    type Error = ContractError;

    fn on<C>(&self, coin: Coin<C>) -> WithCoinResult<Self>
    where
        C: Currency,
    {
        Ok(if currency::equal::<C, FilterC>() {
            SplitCoins {
                filtered: Coin::new(coin.into()),
                rest: Vec::new(),
            }
        } else {
            SplitCoins {
                filtered: Coin::default(),
                rest: vec![coin.into()],
            }
        })
    }
}

struct SplitCoins<FilterC, G>
where
    FilterC: Currency,
    G: Group,
{
    filtered: Coin<FilterC>,
    rest: Vec<CoinDTO<G>>,
}

impl<FilterC, G> Default for SplitCoins<FilterC, G>
where
    FilterC: Currency,
    G: Group,
{
    fn default() -> Self {
        Self {
            filtered: Coin::default(),
            rest: Vec::new(),
        }
    }
}

impl<FilterC, G> Aggregate for SplitCoins<FilterC, G>
where
    FilterC: Currency,
    G: Group,
{
    fn aggregate(self, other: Self) -> Self
    where
        Self: Sized,
    {
        Self {
            filtered: self.filtered + other.filtered,
            rest: self.rest.aggregate(other.rest),
        }
    }
}
