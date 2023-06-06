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

    fn send_nls<B>(
        &self,
        querier: &QuerierWrapper<'_>,
        env: &Env,
        account: B,
    ) -> ContractResult<PlatformResponse>
    where
        B: BankAccount,
    {
        let state_batch = self.enter(env.block.time, querier)?;

        let balance_nls: Coin<Nls> = account.balance()?;

        Ok(if balance_nls.is_zero() {
            PlatformResponse::messages_only(state_batch)
        } else {
            let (bank_batch, bank_emitter) =
                Profit::transfer_nls(account, env, self.config.treasury())?;

            PlatformResponse::messages_with_events(state_batch.merge(bank_batch), bank_emitter)
        })
    }

    fn on_time_alarm(mut self, querier: &QuerierWrapper<'_>, mut env: Env) -> ContractResult<DexResponse<Self>> {
        let contract_addr: Addr = env.contract.address.clone();

        let account: BankStub<BankView<'_>> = bank::account(&contract_addr, querier);

        let balances: Vec<CoinDTO<PaymentGroup>> = account
            .balances::<PaymentGroup, _>(CoinToDTO(PhantomData))?
            .map(never::safe_unwrap)
            .unwrap_or_default();

        match BuyBack::try_new(env.contract.address, self.config, self.account, balances) {
            Ok(buy_back) => Self::try_enter_buy_back(querier, env.block.time, buy_back),
            Err(buy_back::TryNewError {
                contract_addr,
                config,
                account: self_account,
            }) => {
                // Set back moved out values to allow usage of whole object.
                env.contract.address = contract_addr;
                self.config = config;
                self.account = self_account;

                self.send_nls(querier, &env, account)
                    .map(|response: PlatformResponse| (self.into(), response))
            }
        }
        .map(
            |(next_state, response): (State, PlatformResponse)| DexResponse::<Self> {
                response,
                next_state,
            },
        )
    }

    fn try_enter_buy_back(
        querier: &QuerierWrapper<'_>,
        now: Timestamp,
        buy_back: BuyBack,
    ) -> ContractResult<(State, PlatformResponse)> {
        let state: StartLocalLocalState<BuyBack> = dex::start_local_local(buy_back);

        state
            .enter(now, &querier)
            .map(|batch: Batch| {
                (
                    State(StateEnum::BuyBack(state.into())),
                    PlatformResponse::messages_only(batch),
                )
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
