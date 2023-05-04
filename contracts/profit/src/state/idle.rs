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
    batch::{Batch, Emitter},
    message::Response as PlatformResponse,
    never::{self, Never},
};
use sdk::cosmwasm_std::{Deps, Env, QuerierWrapper, Timestamp};

use crate::{msg::ConfigResponse, profit::Profit, result::ContractResult};

use super::{buy_back::BuyBack, Config, ConfigManagement, SetupDexHandler, State, StateEnum};

#[derive(Serialize, Deserialize)]
pub(super) struct Idle {
    config: Config,
    account: Account,
}

impl Idle {
    pub fn new(config: Config, account: Account) -> Self {
        Self { config, account }
    }

    fn send_nls<B>(&self, env: &Env, account: B) -> Result<Option<(Batch, Emitter)>, DexError>
    where
        B: BankAccount,
    {
        let balance_nls: Coin<Nls> = account.balance()?;

        Ok(if balance_nls.is_zero() {
            None
        } else {
            Profit::transfer_nls(account, env, self.config.treasury()).map(Some)?
        })
    }

    fn combine_batches(
        state_batch: Batch,
        bank_batch: Option<(Batch, Emitter)>,
    ) -> PlatformResponse {
        if let Some((bank_batch, bank_emitter)) = bank_batch {
            PlatformResponse::messages_with_events(state_batch.merge(bank_batch), bank_emitter)
        } else {
            PlatformResponse::messages_only(state_batch)
        }
    }

    fn on_time_alarm(self, deps: Deps<'_>, env: Env) -> Result<DexResponse<Self>, DexError> {
        let account: BankStub<BankView<'_>> = bank::account(&env.contract.address, &deps.querier);

        let balances: Vec<CoinDTO<PaymentGroup>> = account
            .balances::<PaymentGroup, _>(CoinToDTO(PhantomData))?
            .map(never::safe_unwrap)
            .unwrap_or_default();

        let bank_batch: Option<(Batch, Emitter)> = self.send_nls(&env, account)?;

        if balances.is_empty() {
            return self
                .enter(env.block.time, &deps.querier)
                .map(|state_batch: Batch| DexResponse::<Self> {
                    response: Self::combine_batches(state_batch, bank_batch),
                    next_state: self.into(),
                });
        }

        let state: StartLocalLocalState<BuyBack> = dex::start_local_local(BuyBack::new(
            env.contract.address,
            self.config,
            self.account,
            balances,
        ));

        let state_batch: Batch = state.enter(env.block.time, &deps.querier)?;

        Ok(DexResponse::<Self> {
            response: Self::combine_batches(state_batch, bank_batch),
            next_state: State(StateEnum::BuyBack(state.into())),
        })
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
    fn try_update_config(self, cadence_hours: u16) -> ContractResult<Self> {
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
    type SwapResult = DexResponse<State>;

    fn on_time_alarm(self, deps: Deps<'_>, env: Env) -> DexResult<Self> {
        match self.on_time_alarm(deps, env) {
            Ok(response) => DexResult::Finished(response),
            Err(error) => DexResult::Continue(Err(error)),
        }
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
