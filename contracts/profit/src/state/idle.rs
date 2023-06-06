use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use currency::{native::Nls, payment::PaymentGroup};
use dex::{
    Account, Enterable, Error as DexError, Handler, Response as DexResponse, Result as DexResult,
    StartLocalLocalState,
};
use finance::{
    coin::{Coin, CoinDTO, WithCoin, WithCoinResult},
    currency::{Currency, Group},
    duration::Duration,
};
use platform::{
    bank::{self, BankAccount, BankAccountView, BankStub, BankView},
    batch::Batch,
    message::Response as PlatformResponse,
    never::{self, Never},
};
use sdk::cosmwasm_std::{Addr, Deps, Env, QuerierWrapper, Timestamp};

use crate::{msg::ConfigResponse, profit::Profit, result::ContractResult};

use super::{
    buy_back::{self, BuyBack},
    CadenceHours, Config, ConfigManagement, SetupDexHandler, State, StateEnum,
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

    fn on_time_alarm(
        mut self,
        querier: &QuerierWrapper<'_>,
        mut env: Env,
    ) -> ContractResult<DexResponse<Self>> {
        let account: BankStub<BankView<'_>> = bank::account(&env.contract.address, querier);

        let balances: Vec<CoinDTO<PaymentGroup>> = account
            .balances::<PaymentGroup, _>(CoinToDTO(PhantomData))?
            .map(never::safe_unwrap)
            .unwrap_or_default();

        self.try_enter_buy_back(querier, env.block.time, balances)
    }

    fn try_enter_buy_back(
        self,
        querier: &QuerierWrapper<'_>,
        now: Timestamp,
        balances: Vec<CoinDTO<PaymentGroup>>,
    ) -> ContractResult<DexResponse<Self>> {
        let state: StartLocalLocalState<BuyBack> = dex::start_local_local(BuyBack::new(
            env.contract.address,
            self.config,
            self.account,
            balances,
        ));

        state
            .enter(now, &querier)
            .map(|batch: Batch| DexResponse::<Self> {
                response: PlatformResponse::messages_only(batch),
                next_state: State(StateEnum::BuyBack(state.into())),
            })
            .map_err(Into::into)
    }
}

impl Enterable for Idle {
    fn enter(&self, now: Timestamp, _: &QuerierWrapper<'_>) -> Result<Batch, DexError> {
        self.config
            .time_alarms()
            .clone()
            .setup_alarm(now + Duration::from_hours(self.config.cadence_hours()))
            .map_err(DexError::TimeAlarmError)
    }
}

impl ConfigManagement for Idle {
    fn try_update_config(self, cadence_hours: CadenceHours) -> ContractResult<Self> {
        Ok(Self {
            config: self.config.update(cadence_hours),
            ..self
        })
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
        DexResult::Finished(self.on_time_alarm(&deps.querier, env))
    }
}

impl SetupDexHandler for Idle {
    type State = Self;
}

struct CoinToDTO<G>(PhantomData<G>)
where
    G: Group;

impl<G> WithCoin for CoinToDTO<G>
where
    G: Group,
{
    type Output = Vec<CoinDTO<G>>;
    type Error = Never;

    fn on<C>(&self, coin: Coin<C>) -> WithCoinResult<Self>
    where
        C: Currency,
    {
        Ok(vec![coin.into()])
    }
}
