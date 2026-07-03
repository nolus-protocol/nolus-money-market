use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    marker::PhantomData,
};

use serde::{Deserialize, Serialize};

use access_control::permissions::SingleUserPermission;
use currencies::{Nls, PaymentGroup};
use currency::{Currency, CurrencyDef, Group, MemberOf};
use dex::{
    Contract, DexResult, Enterable, Error as DexError, Handler, Response as DexResponse,
    Result as SwapDecision,
};
use finance::instant::Instant;
use finance::{
    coin::{Coin, CoinDTO, WithCoin},
    duration::Duration,
};
use platform::{
    bank::{self, Aggregate, BankAccount, BankAccountView},
    batch::Batch,
    message::Response as PlatformResponse,
    state_machine::Response as StateMachineResponse,
};
use sdk::cosmwasm_std::{Env, MessageInfo, QuerierWrapper};
use timealarms::stub::Result as TimeAlarmsResult;

use crate::{CadenceHours, msg::ConfigResponse, profit::Profit, result::ContractResult};
use cw_time::IntoInstant;

use super::{
    Config, ConfigManagement, State, StateEnum, buy_back::BuyBack, resp_delivery::ForwardToDexEntry,
};

#[cfg(all(feature = "internal.test.contract", test))]
mod tests {
    use dex::{
        Account, ConnectionParams, Error as DexError, Handler, Ics20Channel, Result as SwapDecision,
    };
    use remote_profit::callback::{RemoteOperationOutcome, RemoteProfitCallback};
    use remote_profit_wire::profit_id::RemoteProfitId;
    use sdk::cosmwasm_std::{
        self, Addr, Empty, MessageInfo, QuerierWrapper,
        testing::{self, MockQuerier},
    };
    use timealarms::stub::TimeAlarmsRef;

    use crate::state::{Config, State, StateEnum, VaultConfig};

    use super::Idle;

    const CONTROLLER: &str = "controller";
    const STRANGER: &str = "stranger";

    #[test]
    fn late_callback_from_the_controller_is_absorbed() {
        let mock_querier = MockQuerier::<Empty>::default();
        match State::from(idle()).on_remote_profit_callback(
            late_timeout(),
            info(CONTROLLER),
            QuerierWrapper::new(&mock_querier),
            testing::mock_env(),
        ) {
            SwapDecision::Continue(Ok(response)) => {
                assert!(matches!(response.next_state, State(StateEnum::Idle(_))));
            }
            _ => panic!("the late callback should be absorbed as an ok no-op"),
        }
    }

    #[test]
    fn authz_accepts_the_pinned_controller() {
        let mock_querier = MockQuerier::<Empty>::default();
        assert_eq!(
            Ok(()),
            idle()
                .authz_remote_callback(QuerierWrapper::new(&mock_querier), &info(CONTROLLER))
                .map_err(|err| err.to_string())
        );
    }

    #[test]
    fn authz_rejects_a_stranger() {
        let mock_querier = MockQuerier::<Empty>::default();
        assert!(matches!(
            idle().authz_remote_callback(QuerierWrapper::new(&mock_querier), &info(STRANGER)),
            Err(DexError::Unauthorized(_))
        ));
    }

    fn idle() -> Idle {
        Idle::new(config())
    }

    fn late_timeout() -> RemoteProfitCallback {
        RemoteProfitCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationTimeout,
        }
    }

    fn info(sender: &str) -> MessageInfo {
        MessageInfo {
            sender: Addr::unchecked(sender),
            funds: vec![],
        }
    }

    fn config() -> Config {
        Config::new(
            24,
            Addr::unchecked("treasury"),
            oracle_platform::OracleRef::unchecked(Addr::unchecked("oracle")),
            TimeAlarmsRef::unchecked("timealarms"),
            Account::funding(
                Addr::unchecked("profit"),
                ConnectionParams {
                    connection_id: "connection-0".to_owned(),
                    transfer_channel: Ics20Channel {
                        local_endpoint: "channel-0".to_owned(),
                        remote_endpoint: "channel-2048".to_owned(),
                    },
                },
            ),
            Addr::unchecked(CONTROLLER),
            VaultConfig {
                code_id: cosmwasm_std::from_json(b"3").expect("a valid code id"),
                address: Addr::unchecked("drain-vault"),
            },
        )
        .with_profit_authority(profit_authority())
    }

    fn profit_authority() -> RemoteProfitId {
        RemoteProfitId::new("StubPda1111111111111111111111111111".to_owned())
            .expect("a base58 sample")
    }
}

#[derive(Serialize, Deserialize)]
pub(super) struct Idle {
    config: Config,
}

impl Idle {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub(super) fn send_nls<B>(
        &self,
        env: &Env,
        querier: QuerierWrapper<'_>,
        account: B,
        nls: Coin<Nls>,
    ) -> ContractResult<PlatformResponse>
    where
        B: BankAccount,
    {
        self.enter(env.block.time.into_instant(), querier)
            .map(PlatformResponse::messages_only)
            .map(|state_response: PlatformResponse| {
                Profit::transfer_nls(account, self.config.treasury().clone(), nls, env)
                    .merge_with(state_response)
            })
            .map_err(Into::into)
    }

    fn on_time_alarm(
        self,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> ContractResult<DexResponse<Self>> {
        let account = bank::account(&env.contract.address, querier);

        let balances: SplitCoins<Nls, PaymentGroup> = account
            .balances::<PaymentGroup, _>(CoinToDTO(PhantomData, PhantomData))?
            .unwrap_or_default();

        // FM8: with no non-NLS profit to buy back, no remote leg is entered; the
        // cadence is re-armed and any NLS is paid to the treasury locally. The
        // dust filter the buy-back applies — coins below a swappable amount never
        // become legs — is enforced by the remote-swap floor calculator
        // downstream; here the empty-`rest` case short-circuits the remote leg
        // entirely.
        if balances.rest.is_empty() {
            self.send_nls(&env, querier, account, balances.filtered)
                .map(|response: PlatformResponse| DexResponse::<Self> {
                    response,
                    next_state: State(StateEnum::Idle(self)),
                })
        } else {
            self.try_enter_buy_back(querier, env.block.time.into_instant(), balances.rest)
        }
    }

    fn try_enter_buy_back(
        self,
        querier: QuerierWrapper<'_>,
        now: Instant,
        balances: Vec<CoinDTO<PaymentGroup>>,
    ) -> ContractResult<DexResponse<Self>> {
        let start_state = BuyBack::new(self.config, balances).and_then(|spec| {
            dex::start_fund_remote::<_, ForwardToDexEntry>(spec).map_err(Into::into)
        })?;

        start_state
            .enter(now, querier)
            .map(|batch: Batch| DexResponse::<Self> {
                response: PlatformResponse::messages_only(batch),
                next_state: State(StateEnum::FundRemote(start_state.into())),
            })
            .map_err(Into::into)
    }

    fn setup_time_alarm(config: &Config, now: Instant) -> TimeAlarmsResult<Batch> {
        config
            .time_alarms()
            .setup_alarm(now + Duration::from_hours(config.cadence_hours()))
    }
}

impl Enterable for Idle {
    fn enter(&self, now: Instant, _: QuerierWrapper<'_>) -> Result<Batch, DexError> {
        Self::setup_time_alarm(&self.config, now).map_err(DexError::TimeAlarmError)
    }
}

impl Contract for Idle {
    type StateResponse = ConfigResponse;

    fn state(
        self,
        _now: Instant,
        _due_projection: Duration,
        _querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        ConfigResponse {
            cadence_hours: self.config.cadence_hours(),
        }
    }
}

impl ConfigManagement for Idle {
    fn try_update_config(
        self,
        now: Instant,
        cadence_hours: CadenceHours,
    ) -> ContractResult<StateMachineResponse<Self>> {
        let config: Config = self.config.update(cadence_hours);

        Self::setup_time_alarm(&config, now)
            .map(PlatformResponse::messages_only)
            .map(|response: PlatformResponse| StateMachineResponse {
                response,
                next_state: Self { config },
            })
            .map_err(Into::into)
    }
}

impl Handler for Idle {
    type Response = State;
    type SwapResult = ContractResult<DexResponse<State>>;

    /// Authorise the controller pinned in `Config`. `Idle` holds nothing in
    /// flight, so a callback reaching it is a superseded acknowledgment; once
    /// authorised it is swallowed by the absorbing `Handler` defaults rather
    /// than reverting the controller's acknowledgment transaction and stranding
    /// the relayer.
    fn authz_remote_callback(
        &self,
        _querier: QuerierWrapper<'_>,
        info: &MessageInfo,
    ) -> DexResult<()> {
        access_control::check(
            &SingleUserPermission::new(self.config.remote_profit_controller()),
            info,
        )
        .map_err(DexError::Unauthorized)
    }

    fn on_time_alarm(
        self,
        querier: QuerierWrapper<'_>,
        env: Env,
        _info: MessageInfo,
    ) -> SwapDecision<Self> {
        SwapDecision::Finished(self.on_time_alarm(querier, env))
    }
}

impl Display for Idle {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!("Idle"))
    }
}

#[derive(Clone)]
struct CoinToDTO<FilterC, G>(PhantomData<FilterC>, PhantomData<G>)
where
    FilterC: Currency,
    G: Group;

impl<FilterC, G> WithCoin<G> for CoinToDTO<FilterC, G>
where
    FilterC: Currency,
    G: Group,
{
    type Outcome = SplitCoins<FilterC, G>;

    fn on<C>(self, coin: Coin<C>) -> Self::Outcome
    where
        C: CurrencyDef,
        C::Group: MemberOf<G>,
    {
        if currency::equal::<C, FilterC>() {
            SplitCoins {
                filtered: coin.coerce_into(),
                rest: Vec::new(),
            }
        } else {
            SplitCoins {
                filtered: Coin::default(),
                rest: vec![coin.into()],
            }
        }
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
