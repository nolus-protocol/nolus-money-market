use currency::CurrencyDef;
use serde::{Deserialize, Serialize};

use currencies::{Native, Nls, PaymentGroup};
use dex::{
    AcceptAnyNonZeroSwap, Account, AnomalyTreatment, CoinsNb, Connectable, ContractInRemoteSwap,
    ContractInSwap, DexResult, Error as DexError, Response as DexResponse, SlippageEscalation,
    Stage, SwapOutputTask, SwapTask, WithCalculator, WithOutputTask,
};
use finance::instant::Instant;
use finance::{
    coin::{Coin, CoinDTO},
    duration::Duration,
};
use platform::ica::HostAccount;
use remote_profit::{
    msg::SwapParams,
    response::{OperationResponse, SwapResponse},
    stub::{ControllerInnerMessage, Profit as ControllerProfit},
};
use sdk::cosmwasm_std::{self, Addr, Env, MessageInfo, QuerierWrapper};
use timealarms::stub::TimeAlarmsRef;

use crate::{error::ContractError, msg::ConfigResponse, result::ContractResult};

use super::{Config, State, drain::ProfitDrain};

/// A non-`Swap` success acknowledgment can only come from a buggy or hostile
/// counterparty. The fixed reason keeps the unexpected, counterparty-controlled
/// variant out of stored state and events.
const NON_SWAP_RESPONSE: &str = "non-swap operation response";

/// The acknowledged output currency is not NLS, so the response cannot have
/// originated from the scheduled buy-back swap.
const OUT_NOT_NLS: &str = "swapped-out currency is not NLS";

const TIMEOUT_RETRY_BUDGET: CoinsNb = 3;

#[derive(Serialize, Deserialize)]
pub(super) struct BuyBack {
    config: Config,
    coins: Vec<CoinDTO<PaymentGroup>>,
    /// The Solana profit authority the funding transfers are addressed to,
    /// bridged from the learned authority. Resolved once at construction, so a
    /// cycle that reached here proved the authority was already learned.
    funding_receiver: HostAccount,
}

impl BuyBack {
    pub fn new(config: Config, coins: Vec<CoinDTO<PaymentGroup>>) -> ContractResult<Self> {
        debug_assert!(
            coins
                .iter()
                .all(|not_native: &CoinDTO<PaymentGroup>| not_native
                    .of_currency_dto(Nls::dto())
                    .is_err()),
            "{coins:?}",
        );

        config.funding_receiver().map(|funding_receiver| Self {
            config,
            coins,
            funding_receiver,
        })
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
        self.config.account()
    }

    fn time_alarm(&self) -> &TimeAlarmsRef {
        self.config.time_alarms()
    }

    /// Authorised against the controller pinned in `Config`: only the
    /// remote-profit controller can advance the in-flight swap leg.
    fn authz_remote_callback(
        &self,
        _querier: QuerierWrapper<'_>,
        info: &MessageInfo,
    ) -> DexResult<()> {
        access_control::check(
            &access_control::permissions::SingleUserPermission::new(
                self.config.remote_profit_controller(),
            ),
            info,
        )
        .map_err(DexError::Unauthorized)
    }

    /// The buy-back parks at the slippage-anomaly terminal on an under-floor
    /// rejection and is re-driven by an operator heal; it never opens a
    /// permissionless anomaly-resolution path.
    fn authz_anomaly_resolution(
        &self,
        _querier: QuerierWrapper<'_>,
        _info: &MessageInfo,
    ) -> DexResult<()> {
        Err(DexError::Unauthorized(
            access_control::error::Error::Unauthorized {},
        ))
    }

    fn timeout_retry_budget(&self) -> CoinsNb {
        TIMEOUT_RETRY_BUDGET
    }

    fn slippage_escalation(&self) -> SlippageEscalation {
        SlippageEscalation::Park
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
        amount_out: Coin<Self::OutC>,
        env: &Env,
        querier: QuerierWrapper<'_>,
    ) -> <Self as SwapTask>::Result {
        ProfitDrain::new(self.config, amount_out, querier)
            .and_then(|drain| drain.start(env, querier))
    }
}

impl dex::RemoteSwapClient for BuyBack {
    fn schedule_swap(
        &self,
        coin_in: &CoinDTO<Self::InG>,
        min_out: &CoinDTO<Self::OutG>,
        nonce: u64,
    ) -> DexResult<platform::batch::Batch> {
        SwapParams::new(*coin_in, min_out.into_super_group())
            .map_err(DexError::remote_swap_client)
            .and_then(|params| {
                ControllerProfit::new(self.config.remote_profit_controller())
                    .swap(params, SwapParams::TIMEOUT, |params, timeout| {
                        ControllerExecuteMsg::Swap {
                            params,
                            timeout,
                            nonce,
                        }
                    })
                    .map_err(Into::into)
            })
    }

    fn decode_response(&self, payload: &[u8]) -> DexResult<CoinDTO<Self::OutG>> {
        cosmwasm_std::from_json::<OperationResponse>(payload)
            .map_err(DexError::remote_swap_client)
            .and_then(|response| match response {
                OperationResponse::Swap(SwapResponse { amount_out }) => {
                    Coin::<Nls>::try_from(amount_out)
                        .map(Into::into)
                        .map_err(|_not_nls| DexError::unexpected_response_variant(OUT_NOT_NLS))
                }
                OperationResponse::OpenProfit(_)
                | OperationResponse::CloseProfit(_)
                | OperationResponse::TransferOut(_) => {
                    Err(DexError::unexpected_response_variant(NON_SWAP_RESPONSE))
                }
            })
    }

    /// The buy-back never opts into the zero-acked unwind, so this path is
    /// unreachable; it returns a visible error rather than driving an unwind the
    /// profit flow has no inputs to drain.
    fn unwind(self, _querier: QuerierWrapper<'_>, _env: &Env) -> <Self as SwapTask>::Result {
        Err(ContractError::unsupported_operation(
            "the buy-back swap does not unwind on a zero-acked error",
        ))
    }
}

impl dex::FundingClient for BuyBack {
    fn funding_sender(&self) -> &Addr {
        self.dex_account().owner()
    }

    fn funding_receiver(&self) -> &HostAccount {
        &self.funding_receiver
    }

    fn transfer_channel(&self) -> &str {
        self.dex_account()
            .dex()
            .transfer_channel
            .local_endpoint
            .as_str()
    }
}

impl ContractInSwap for BuyBack {
    type StateResponse = <Self as SwapTask>::StateResponse;

    fn state(
        self,
        _in_progress: Stage,
        _now: Instant,
        _due_projection: Duration,
        _querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        ConfigResponse {
            cadence_hours: self.config.cadence_hours(),
        }
    }
}

impl ContractInRemoteSwap for BuyBack {
    type StateResponse = <Self as SwapTask>::StateResponse;

    fn state(
        self,
        _acks_left: CoinsNb,
        _now: Instant,
        _due_projection: Duration,
        _querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        ConfigResponse {
            cadence_hours: self.config.cadence_hours(),
        }
    }

    fn anomaly_response(
        self,
        _acks_left: CoinsNb,
        _now: Instant,
        _due_projection: Duration,
        _querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        ConfigResponse {
            cadence_hours: self.config.cadence_hours(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum ControllerExecuteMsg {
    Swap {
        params: SwapParams,
        timeout: Duration,
        nonce: u64,
    },
}

impl ControllerInnerMessage for ControllerExecuteMsg {}

#[cfg(all(feature = "internal.test.contract", test))]
mod tests {
    use currencies::testing::PaymentC3;
    use dex::{Account, ConnectionParams, Ics20Channel, SwapTask};
    use finance::coin::Coin;
    use remote_profit_wire::profit_id::RemoteProfitId;
    use sdk::cosmwasm_std::{self, Addr};
    use timealarms::stub::TimeAlarmsRef;

    use crate::state::VaultConfig;

    use super::{BuyBack, Config};

    /// Truth table (#660): the profit buy-back runs `AcceptAnyNonZeroSwap`
    /// and keeps the verbatim re-emission class — `requote_on_timeout` stays
    /// at its `false` default.
    /// COMPILE-RED: blocked on `SwapTask::requote_on_timeout`.
    #[test]
    fn buy_back_does_not_requote_on_timeout() {
        assert!(!spec().requote_on_timeout());
    }

    fn spec() -> BuyBack {
        BuyBack::new(config(), vec![Coin::<PaymentC3>::new(1_000).into()])
            .expect("the buy-back spec should build")
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
            Addr::unchecked("controller"),
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
