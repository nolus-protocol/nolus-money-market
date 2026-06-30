use std::iter;

use serde::{Deserialize, Serialize};

use access_control::permissions::SingleUserPermission;
use currencies::{Native, Nls};
use cw_time::IntoInstant;
use dex::{DrainStage, Enterable, Error as DexError, RemoteTransferOutTask};
use finance::{
    coin::{Coin, CoinDTO},
    duration::Duration,
    instant::Instant,
};
use platform::{bank, batch::Batch, message::Response as PlatformResponse};
use remote_profit::{
    msg::TransferOutParams,
    response::OperationResponse,
    stub::{ControllerInnerMessage, Profit as ControllerProfit},
};
use sdk::cosmwasm_std::{self, Addr, Env, MessageInfo, QuerierWrapper};
use timealarms::stub::TimeAlarmsRef;

use crate::{error::ContractError, msg::ConfigResponse, result::ContractResult, state::State};

use super::{Config, StateEnum, arrival, idle::Idle};

/// A non-`TransferOut` success acknowledgment can only come from a buggy or
/// hostile counterparty. The fixed reason keeps the unexpected,
/// counterparty-controlled variant out of stored state and events.
const NON_TRANSFER_OUT_RESPONSE: &str = "non-transfer-out operation response";

/// The home-bound drain of the bought-back NLS proceeds
///
/// The accumulated NLS bought back on Solana is transferred out, over the
/// remote-profit controller, into the dedicated `drain_vault`, then its arrival
/// in that vault is awaited. The vault is the FM1 arrival-isolation account: it
/// never receives passive NLS, so the balance-baseline arrival gate stays sound
/// — a lease repayment landing on the profit account mid-drain cannot satisfy
/// it. On arrival the vault is swept back into the profit account and the
/// proceeds are paid to the treasury.
#[derive(Serialize, Deserialize)]
pub(crate) struct ProfitDrain {
    config: Config,
    proceeds: Coin<Nls>,
    /// The single drain-vault account that drives both the arrival poll
    /// (`arrival_account`) and the entry baseline (`snapshot_baseline`). One
    /// stored `Addr` makes the FM1 baseline-equals-poll invariant structural:
    /// the snapshot and the poll cannot diverge.
    drain_vault: Addr,
    /// The vault balance, in NLS, at drain entry — before any coin was drained
    /// home. The arrival check measures against this baseline, never an absolute
    /// balance, so a balance the vault already held cannot be mistaken for the
    /// proceeds arriving.
    baseline: Vec<CoinDTO<Native>>,
}

impl ProfitDrain {
    pub(super) fn new(
        config: Config,
        proceeds: Coin<Nls>,
        querier: QuerierWrapper<'_>,
    ) -> ContractResult<Self> {
        let drain_vault = config.drain_vault().clone();
        let proceeds_dto: CoinDTO<Native> = proceeds.into();
        arrival::snapshot_baseline(&[proceeds_dto], &drain_vault, querier)
            .map_err(ContractError::from)
            .map(|baseline| Self {
                config,
                proceeds,
                drain_vault,
                baseline,
            })
    }

    pub(super) fn start(
        self,
        env: &Env,
        querier: QuerierWrapper<'_>,
    ) -> ContractResult<dex::Response<State>> {
        let start_drain = dex::start_drain(self)?;
        start_drain
            .enter(env.block.time.into_instant(), querier)
            .map(|drain_msgs| {
                dex::Response::<State>::from(
                    PlatformResponse::messages_only(drain_msgs),
                    State(StateEnum::Drain(dex::StateDrain::from(start_drain))),
                )
            })
            .map_err(Into::into)
    }

    fn proceeds_dto(&self) -> CoinDTO<Native> {
        self.proceeds.into()
    }
}

impl RemoteTransferOutTask for ProfitDrain {
    type G = Native;
    type Label = String;
    type StateResponse = ConfigResponse;
    type Result = ContractResult<dex::Response<State>>;

    fn label(&self) -> Self::Label {
        String::from("ProfitDrain")
    }

    fn time_alarm(&self) -> &TimeAlarmsRef {
        self.config.time_alarms()
    }

    /// Authorised against the controller pinned in `Config` — the funding and
    /// swap phases authorised the same controller's callback, so a controller
    /// re-configuration can neither wedge nor hijack the proceeds drain.
    fn authz_remote_callback(
        &self,
        _querier: QuerierWrapper<'_>,
        info: &MessageInfo,
    ) -> dex::DexResult<()> {
        access_control::check(
            &SingleUserPermission::new(self.config.remote_profit_controller()),
            info,
        )
        .map_err(DexError::Unauthorized)
    }

    fn coins(&self) -> impl IntoIterator<Item = CoinDTO<Self::G>> {
        iter::once(self.proceeds_dto())
    }

    fn schedule_transfer_out(&self, coin: &CoinDTO<Self::G>, nonce: u64) -> dex::DexResult<Batch> {
        transfer_out_msg(self.config.remote_profit_controller(), coin, nonce)
    }

    fn decode_response(&self, payload: &[u8]) -> dex::DexResult<()> {
        decode_response(payload)
    }

    /// FM1: the arrival gate polls the dedicated drain vault, the same account
    /// `new` snapshotted the baseline against. The fail-closed assert makes the
    /// baseline-equals-poll invariant executable: a refactor that let the two
    /// diverge trips here in debug/test.
    fn arrival_account<'arrival>(&'arrival self, _contract: &'arrival Addr) -> &'arrival Addr {
        &self.drain_vault
    }

    fn all_received(&self, account: &Addr, querier: QuerierWrapper<'_>) -> dex::DexResult<bool> {
        debug_assert_eq!(
            account, &self.drain_vault,
            "the arrival poll must read the same account the baseline was snapshotted against",
        );
        arrival::arrived_over_baseline(&[self.proceeds_dto()], &self.baseline, account, querier)
    }

    fn finish(self, env: &Env, querier: QuerierWrapper<'_>) -> Self::Result {
        let profit_addr = env.contract.address.clone();
        sweep_vault(&self.drain_vault, &profit_addr)
            .map_err(ContractError::from)
            .and_then(|sweep_msgs| {
                let next_state = Idle::new(self.config);
                let profit_account = bank::account(&profit_addr, querier);

                next_state
                    .send_nls(env, querier, profit_account, self.proceeds)
                    .map(|payout| {
                        dex::Response::<State>::from(
                            PlatformResponse::messages_only(sweep_msgs).merge_with(payout),
                            State(StateEnum::Idle(next_state)),
                        )
                    })
            })
    }

    fn state(
        self,
        _in_progress: DrainStage,
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
    TransferOut {
        params: TransferOutParams,
        timeout: Duration,
        nonce: u64,
    },
}

impl ControllerInnerMessage for ControllerExecuteMsg {}

fn transfer_out_msg(
    controller: &Addr,
    coin: &CoinDTO<Native>,
    nonce: u64,
) -> dex::DexResult<Batch> {
    TransferOutParams::new(coin.into_super_group())
        .map_err(DexError::remote_swap_client)
        .and_then(|params| {
            ControllerProfit::new(controller)
                .transfer_out(params, TransferOutParams::TIMEOUT, |params, timeout| {
                    ControllerExecuteMsg::TransferOut {
                        params,
                        timeout,
                        nonce,
                    }
                })
                .map_err(Into::into)
        })
}

fn decode_response(payload: &[u8]) -> dex::DexResult<()> {
    cosmwasm_std::from_json::<OperationResponse>(payload)
        .map_err(DexError::remote_swap_client)
        .and_then(|response| match response {
            OperationResponse::TransferOut(_confirmation) => Ok(()),
            OperationResponse::OpenProfit(_)
            | OperationResponse::CloseProfit(_)
            | OperationResponse::Swap(_) => Err(DexError::unexpected_response_variant(
                NON_TRANSFER_OUT_RESPONSE,
            )),
        })
}

fn sweep_vault(vault: &Addr, recipient: &Addr) -> Result<Batch, platform::error::Error> {
    let mut batch = Batch::default();
    batch
        .schedule_execute_wasm_no_reply_no_funds(
            vault.clone(),
            &drain_vault::api::ExecuteMsg::Sweep {
                recipient: recipient.clone(),
            },
        )
        .map(|()| batch)
}

#[cfg(all(feature = "internal.test.contract", test))]
mod tests {
    use currencies::{Native, Nls};
    use currency::CurrencyDef;
    use finance::coin::{Coin, CoinDTO};
    use sdk::cosmwasm_std::{
        self, Addr, Coin as CwCoin, Empty, QuerierWrapper, testing::MockQuerier,
    };

    const CONTROLLER: &str = "controller";
    const VAULT: &str = "drain-vault";
    const PROFIT: &str = "profit";
    const PROCEEDS: u128 = 1_000;

    #[test]
    fn transfer_out_msg_targets_the_controller() {
        let coin: CoinDTO<Native> = Coin::<Nls>::new(PROCEEDS).into();
        let batch = super::transfer_out_msg(&Addr::unchecked(CONTROLLER), &coin, 7)
            .expect("a valid transfer-out message");
        assert_eq!(1, batch.len());
    }

    #[test]
    fn decode_accepts_a_transfer_out_response() {
        use remote_profit::response::{OperationResponse, TransferOutResponse};
        let payload = OperationResponse::TransferOut(TransferOutResponse {});
        let bytes = cosmwasm_std::to_json_vec(&payload).expect("the response serializes");
        assert_eq!(
            Ok(()),
            super::decode_response(&bytes).map_err(|e| e.to_string())
        );
    }

    #[test]
    fn decode_rejects_non_transfer_out_responses() {
        use remote_profit::response::{CloseProfitResponse, OperationResponse};
        let payload = OperationResponse::CloseProfit(CloseProfitResponse {});
        let bytes = cosmwasm_std::to_json_vec(&payload).expect("the response serializes");
        assert!(matches!(
            super::decode_response(&bytes),
            Err(dex::Error::UnexpectedResponseVariant(_reason))
        ));
    }

    /// FM1: the baseline is snapshotted against the drain vault, and the
    /// arrival poll reads that same vault — so an inbound NLS send to the
    /// PROFIT account (not the vault) never satisfies the gate. The vault is
    /// distinct from the profit address.
    #[test]
    fn baseline_and_poll_share_the_vault_account() {
        let vault = Addr::unchecked(VAULT);
        let profit = Addr::unchecked(PROFIT);
        assert_ne!(vault, profit);

        let proceeds: CoinDTO<Native> = Coin::<Nls>::new(PROCEEDS).into();
        // The vault holds nothing at entry; the profit account holds the
        // proceeds-worth of NLS already (its passive reserve).
        let entry = held(&[(VAULT, 0), (PROFIT, PROCEEDS)]);
        let baseline =
            super::arrival::snapshot_baseline(&[proceeds], &vault, QuerierWrapper::new(&entry))
                .expect("the baseline snapshots against the vault");

        // The profit account now holds even more NLS (a lease repayment landed),
        // but the vault still holds nothing — the gate must NOT complete.
        let mid = held(&[(VAULT, 0), (PROFIT, PROCEEDS + PROCEEDS)]);
        assert_eq!(
            Ok(false),
            super::arrival::arrived_over_baseline(
                &[proceeds],
                &baseline,
                &vault,
                QuerierWrapper::new(&mid),
            )
            .map_err(|e| e.to_string())
        );

        // Only once the proceeds land in the VAULT does the gate complete.
        let arrived = held(&[(VAULT, PROCEEDS), (PROFIT, PROCEEDS + PROCEEDS)]);
        assert_eq!(
            Ok(true),
            super::arrival::arrived_over_baseline(
                &[proceeds],
                &baseline,
                &vault,
                QuerierWrapper::new(&arrived),
            )
            .map_err(|e| e.to_string())
        );
    }

    fn held(holdings: &[(&str, u128)]) -> MockQuerier<Empty> {
        let entries: Vec<(&str, Vec<CwCoin>)> = holdings
            .iter()
            .map(|(addr, amount)| {
                (
                    *addr,
                    vec![CwCoin::new(*amount, Nls::dto().definition().bank_symbol)],
                )
            })
            .collect();
        let refs: Vec<(&str, &[CwCoin])> = entries
            .iter()
            .map(|(addr, coins)| (*addr, coins.as_slice()))
            .collect();
        MockQuerier::<Empty>::new(&refs)
    }
}
