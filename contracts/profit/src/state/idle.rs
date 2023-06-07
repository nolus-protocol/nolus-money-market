use serde::{Deserialize, Serialize};

use currency::{
    native::{Native, Nls},
    payment::PaymentGroup,
};
use dex::{
    Account, Enterable, Error as DexError, Handler, Response as DexResponse, Result as DexResult,
    StartLocalLocalState,
};
use finance::{
    coin::{Coin, CoinDTO, WithCoin, WithCoinResult},
    currency::{
        Currency,
        Group,
        equal
    },
    duration::Duration,
};
use platform::{
    bank::{self, Aggregate, BankAccount, BankAccountView, BankStub, BankView},
    batch::Batch,
    message::Response as PlatformResponse,
};
use sdk::cosmwasm_std::{Addr, Deps, Env, QuerierWrapper, Timestamp};

use crate::{msg::ConfigResponse, profit::Profit, result::ContractResult, ContractError};

use super::{
    buy_back::{BuyBack, BuyBackCurrencies},
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
        env: &Env,
        querier: &QuerierWrapper<'_>,
        account: B,
        native: Option<CoinDTO<Native>>,
    ) -> ContractResult<PlatformResponse>
    where
        B: BankAccount,
    {
        let state_response: PlatformResponse =
            PlatformResponse::messages_only(self.enter(env.block.time, querier)?);

        let nls: Option<Coin<Nls>> =
            native.and_then(|coin_dto: CoinDTO<Native>| coin_dto.try_into().ok());

        Ok(if let Some(nls) = nls {
            Profit::transfer_nls(account, self.config.treasury(), nls, env)
                .merge_with(state_response)
        } else {
            state_response
        })
    }

    fn on_time_alarm(
        self,
        querier: &QuerierWrapper<'_>,
        env: Env,
    ) -> ContractResult<DexResponse<Self>> {
        let account: BankStub<BankView<'_>> = bank::account(&env.contract.address, querier);

        let mut balances: SplitCoins<Native, BuyBackCurrencies> = account
            .balances::<PaymentGroup, _>(CoinToDTO)?
            .transpose()?
            .unwrap_or_default();

        if !balances.second.is_empty() {
            self.try_enter_buy_back(
                querier,
                env.contract.address,
                env.block.time,
                balances.second,
            )
        } else if balances.first.len() <= 1 {
            self.send_nls(&env, querier, account, balances.first.pop())
                .map(|response: PlatformResponse| DexResponse::<Self> {
                    response,
                    next_state: State(StateEnum::Idle(self)),
                })
        } else {
            Err(ContractError::BuybackBrokenInvariant(String::from(
                "More than one entry in native currencies list encountered!"
            )))
        }
    }

    fn try_enter_buy_back(
        self,
        querier: &QuerierWrapper<'_>,
        profit_addr: Addr,
        now: Timestamp,
        balances: Vec<CoinDTO<BuyBackCurrencies>>,
    ) -> ContractResult<DexResponse<Self>> {
        let state: StartLocalLocalState<BuyBack> = dex::start_local_local(BuyBack::new(
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

struct CoinToDTO;

impl WithCoin for CoinToDTO {
    type Output = SplitCoins<Native, BuyBackCurrencies>;
    type Error = ContractError;

    fn on<C>(&self, coin: Coin<C>) -> WithCoinResult<Self>
    where
        C: Currency,
    {
        if equal::<C, Nls>() {
            Ok(SplitCoins {
                first: vec![coin.into()],
                second: Vec::new(),
            })
        } else if BuyBackCurrencies::contains::<C>() {
            Ok(SplitCoins {
                first: Vec::new(),
                second: vec![coin.into()],
            })
        } else {
            Err(ContractError::BuybackUnrecognisedCurrency(C::TICKER))
        }
    }
}

struct SplitCoins<G1, G2>
where
    G1: Group,
    G2: Group,
{
    first: Vec<CoinDTO<G1>>,
    second: Vec<CoinDTO<G2>>,
}

impl<G1, G2> Default for SplitCoins<G1, G2>
where
    G1: Group,
    G2: Group,
{
    fn default() -> Self {
        Self {
            first: vec![],
            second: vec![],
        }
    }
}

impl<G1, G2> Aggregate for SplitCoins<G1, G2>
where
    G1: Group,
    G2: Group,
{
    fn aggregate(self, other: Self) -> Self
    where
        Self: Sized,
    {
        Self {
            first: self.first.aggregate(other.first),
            second: self.second.aggregate(other.second),
        }
    }
}
