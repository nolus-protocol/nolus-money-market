use serde::{Deserialize, Serialize};

use access_control::permissions::SingleUserPermission;
use currency::{CurrencyDef, Group, MemberOf};
use dex::{DrainStage, Error as DexError, RemoteTransferOutTask};
use finance::{
    coin::{Amount, Coin, CoinDTO, WithCoin},
    duration::Duration,
    instant::Instant,
    zero::Zero,
};
use platform::{bank, batch::Batch, error::Error as PlatformError};
use remote_lease::{
    msg::TransferOutParams,
    response::OperationResponse,
    stub::{ControllerInnerMessage, Lease as ControllerLease},
};
use sdk::cosmwasm_std::{self, Addr, Env, MessageInfo, QuerierWrapper};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{
        DownpaymentCoin, LeasePaymentCurrencies,
        open::NewLeaseForm,
        query::{StateResponse as QueryStateResponse, opening::OngoingTrx},
    },
    contract::{
        cmd::OpenLoanRespResult,
        finalize::LeasesRef,
        state::{
            SwapResult,
            opening::refund::{OpenFailureRefund, refund_to_open_failed},
        },
    },
    error::{ContractError, ContractResult},
    event::Type,
    finance::{LpnCurrency, LppRef, OracleRef, ReserveRef},
};

/// A non-`TransferOut` success acknowledgment can only come from a buggy
/// or hostile counterparty. The fixed reason keeps the unexpected,
/// counterparty-controlled variant out of stored state and events.
const NON_TRANSFER_OUT_RESPONSE: &str = "non-transfer-out operation response";

/// The open-failure reason a clean unwind records: the opening swap was
/// rejected before any leg acknowledged, so the inputs were drained home.
const UNWIND_REASON: &str = "opening swap unwound";

/// Drain a failed opening swap's inputs home and refund
///
/// Entered when the opening swap hits a hard remote error with nothing
/// acknowledged yet (`total_out == 0`). The downpayment and the loan principal
/// are still wholly on the Solana-side `LeaseAuthority`, so this drains them
/// back over the same transfer-out transport the paid-close drain uses, then
/// refunds the customer and repays the LPP loan in full, ending in
/// `OpenFailed`.
#[derive(Serialize, Deserialize)]
pub(crate) struct OpeningUnwindTask {
    form: NewLeaseForm,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    lpp_ref: LppRef,
    leases_ref: LeasesRef,
    time_alarms: TimeAlarmsRef,
    remote_lease_controller: Addr,
    /// The local-account balance, per drained currency, at the moment the
    /// unwind was entered — before any coin had been drained back. The arrival
    /// check measures against this baseline, never an absolute balance, so a
    /// balance the lease already held, or a single same-currency leg, cannot
    /// trigger a premature refund. Persisted with the task so it survives every
    /// callback's serde round-trip; it is captured once at construction and
    /// never recomputed.
    baseline: Vec<DownpaymentCoin>,
}

impl OpeningUnwindTask {
    /// Build the unwind task, snapshotting the local-account baseline
    ///
    /// Must run synchronously while the failed opening error is handled, before
    /// any drain transfer dispatches, so the snapshot reflects the pre-drain
    /// balances.
    pub(in super::super) fn enter(
        form: NewLeaseForm,
        downpayment: DownpaymentCoin,
        loan: OpenLoanRespResult,
        deps: (LppRef, OracleRef, TimeAlarmsRef, LeasesRef),
        remote_lease_controller: Addr,
        lease_addr: &Addr,
        querier: QuerierWrapper<'_>,
    ) -> ContractResult<Self> {
        let (lpp_ref, _oracle, time_alarms, leases_ref) = deps;
        let principal_in = loan.principal.into_super_group::<LeasePaymentCurrencies>();
        let baseline = unique_currencies([downpayment, principal_in])
            .map(|coin| account_balance(&coin, lease_addr, querier))
            .collect::<Result<Vec<_>, PlatformError>>()
            .map_err(ContractError::from)?;
        Ok(Self {
            form,
            downpayment,
            loan,
            lpp_ref,
            leases_ref,
            time_alarms,
            remote_lease_controller,
            baseline,
        })
    }

    fn drained(&self) -> [DownpaymentCoin; 2] {
        [
            self.downpayment,
            self.loan
                .principal
                .into_super_group::<LeasePaymentCurrencies>(),
        ]
    }

    fn state_envelope(&self) -> ContractResult<QueryStateResponse> {
        Ok(QueryStateResponse::Opening {
            currency: self.form.currency,
            downpayment: self.downpayment,
            loan: self.loan.principal,
            loan_interest_rate: self.loan.annual_interest_rate,
            in_progress: OngoingTrx::Unwinding,
        })
    }
}

impl RemoteTransferOutTask for OpeningUnwindTask {
    type G = LeasePaymentCurrencies;
    type Label = Type;
    type StateResponse = ContractResult<QueryStateResponse>;
    type Result = SwapResult;

    fn label(&self) -> Self::Label {
        Type::OpeningUnwind
    }

    fn time_alarm(&self) -> &TimeAlarmsRef {
        &self.time_alarms
    }

    /// Authorised against the controller the opening pinned — the same one the
    /// swap leg's callback authorised, so a leaser re-configuration mid-unwind
    /// can neither wedge nor hijack the drain.
    fn authz_remote_callback(
        &self,
        _querier: QuerierWrapper<'_>,
        info: &MessageInfo,
    ) -> dex::DexResult<()> {
        access_control::check(
            &SingleUserPermission::new(&self.remote_lease_controller),
            info,
        )
        .map_err(DexError::Unauthorized)
    }

    fn coins(&self) -> impl IntoIterator<Item = CoinDTO<Self::G>> {
        self.drained()
    }

    fn schedule_transfer_out(&self, coin: &CoinDTO<Self::G>) -> dex::DexResult<Batch> {
        transfer_out_msg(&self.remote_lease_controller, coin)
    }

    fn decode_response(&self, payload: &[u8]) -> dex::DexResult<()> {
        decode_response(payload)
    }

    /// Every drained coin has arrived once each currency's measured balance has
    /// risen above its entry baseline by at least the aggregate amount expected
    /// in that currency. Two properties make this safe where an absolute
    /// balance check is not. The baseline is subtracted, so a balance the lease
    /// already held — including one an attacker bank-sent to force an early
    /// finish — is not mistaken for an arrival. And coins are grouped by
    /// currency and summed, so a downpayment and a principal in the same
    /// currency must BOTH land before the check passes.
    fn all_received(&self, account: &Addr, querier: QuerierWrapper<'_>) -> dex::DexResult<bool> {
        unique_currencies(self.drained()).try_fold(true, |all_received, expected_currency| {
            let aggregate = aggregate_amount(self.drained(), &expected_currency);
            let baseline = aggregate_amount(self.baseline.iter().copied(), &expected_currency);
            account_balance(&expected_currency, account, querier)
                .map_err(Into::into)
                .map(|arrived| {
                    all_received && arrived.amount().saturating_sub(baseline) >= aggregate
                })
        })
    }

    fn finish(self, env: &Env, querier: QuerierWrapper<'_>) -> Self::Result {
        let now = cw_time::IntoInstant::into_instant(env.block.time);
        let lease_addr = env.contract.address.clone();
        Coin::<LpnCurrency>::try_from(self.loan.principal)
            .map_err(Into::into)
            .and_then(|principal| {
                ReserveRef::try_new(self.form.reserve.clone(), &querier)
                    .map_err(Into::into)
                    .map(|reserve| (principal, reserve))
            })
            .and_then(|(principal, reserve)| {
                refund_to_open_failed(
                    OpenFailureRefund {
                        downpayment: self.downpayment,
                        principal,
                        customer: self.form.customer,
                        reserve,
                        lpp_ref: self.lpp_ref,
                        leases_ref: self.leases_ref,
                        lease_addr,
                        now,
                    },
                    remote_lease::callback::RemoteErrorMessage::from_static(UNWIND_REASON),
                    querier,
                )
            })
    }

    fn state(
        self,
        _in_progress: DrainStage,
        _now: Instant,
        _due_projection: Duration,
        _querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        self.state_envelope()
    }
}

/// Deduplicate a coin list down to one representative per currency
///
/// The drain handles at most two coins, so the linear scan is trivial; the
/// representative carries the currency only — its amount is irrelevant to the
/// per-currency aggregation done by [`aggregate_amount`].
fn unique_currencies(
    coins: impl IntoIterator<Item = DownpaymentCoin>,
) -> impl Iterator<Item = DownpaymentCoin> {
    let mut seen: Vec<DownpaymentCoin> = Vec::new();
    for coin in coins {
        if !seen.iter().any(|kept| kept.currency() == coin.currency()) {
            seen.push(coin);
        }
    }
    seen.into_iter()
}

/// Sum the amounts of every coin matching `currency`
fn aggregate_amount(
    coins: impl IntoIterator<Item = DownpaymentCoin>,
    currency: &DownpaymentCoin,
) -> Amount {
    coins
        .into_iter()
        .filter(|coin| coin.currency() == currency.currency())
        .map(|coin| coin.amount())
        .fold(Amount::ZERO, Amount::saturating_add)
}

/// Snapshot the local-account balance in `coin`'s currency
///
/// Returns the raw bank error so each caller maps it into its own error type:
/// the entry snapshot into `ContractError`, the arrival check into `dex::Error`.
fn account_balance(
    coin: &DownpaymentCoin,
    account: &Addr,
    querier: QuerierWrapper<'_>,
) -> Result<DownpaymentCoin, PlatformError> {
    struct Balance<'account, 'querier> {
        account: &'account Addr,
        querier: QuerierWrapper<'querier>,
    }

    impl WithCoin<LeasePaymentCurrencies> for Balance<'_, '_> {
        type Outcome = Result<DownpaymentCoin, PlatformError>;

        fn on<C>(self, _coin: Coin<C>) -> Self::Outcome
        where
            C: CurrencyDef,
            C::Group: MemberOf<LeasePaymentCurrencies>
                + MemberOf<<LeasePaymentCurrencies as Group>::TopG>,
        {
            bank::balance::<C>(self.account, self.querier).map(CoinDTO::from)
        }
    }

    coin.with_coin(Balance { account, querier })
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum ControllerExecuteMsg {
    TransferOut {
        params: TransferOutParams,
        timeout: Duration,
    },
}

impl ControllerInnerMessage for ControllerExecuteMsg {}

fn transfer_out_msg(
    controller: &Addr,
    coin: &CoinDTO<LeasePaymentCurrencies>,
) -> dex::DexResult<Batch> {
    TransferOutParams::new(*coin)
        .map_err(DexError::remote_swap_client)
        .and_then(|params| {
            ControllerLease::new(controller)
                .transfer_out(params, TransferOutParams::TIMEOUT, |params, timeout| {
                    ControllerExecuteMsg::TransferOut { params, timeout }
                })
                .map_err(Into::into)
        })
}

fn decode_response(payload: &[u8]) -> dex::DexResult<()> {
    cosmwasm_std::from_json::<OperationResponse>(payload)
        .map_err(DexError::remote_swap_client)
        .and_then(|response| match response {
            OperationResponse::TransferOut(_confirmation) => Ok(()),
            OperationResponse::OpenLease(_)
            | OperationResponse::CloseLease(_)
            | OperationResponse::Swap(_) => Err(DexError::unexpected_response_variant(
                NON_TRANSFER_OUT_RESPONSE,
            )),
        })
}

#[cfg(all(feature = "internal.test.contract", test))]
mod tests {
    use currencies::{
        Lpn,
        testing::{PaymentC1, PaymentC2},
    };
    use currency::CurrencyDef;
    use dex::RemoteTransferOutTask;
    use finance::{
        coin::{Amount, Coin},
        duration::Duration,
        percent::Percent100,
    };
    use lpp::stub::LppRef as LppGenericRef;
    use sdk::cosmwasm_std::{Addr, Coin as CwCoin, Empty, QuerierWrapper, testing::MockQuerier};
    use timealarms::stub::TimeAlarmsRef;

    use crate::{
        api::{
            DownpaymentCoin,
            open::{LoanForm, NewLeaseForm, PositionSpecDTO},
        },
        contract::{cmd::OpenLoanRespResult, finalize::LeasesRef},
        finance::{LpnCurrency, OracleRef},
    };

    use super::OpeningUnwindTask;

    const LEASE: &str = "lease";
    const DOWNPAYMENT: Amount = 1_000;
    const PRINCIPAL: Amount = 4_000;

    /// The baseline is persisted, not `#[serde(skip)]`: it must survive every
    /// callback's serde round-trip, since a baseline reset to a post-arrival
    /// balance would make the absolute-vs-baseline distinction collapse and
    /// release funds early.
    #[test]
    fn baseline_survives_serde_round_trip() {
        let task = distinct_currency_task(&balances(&[]));

        let serialized = sdk::cosmwasm_std::to_json_vec(&task).expect("a serializable task");
        let restored: OpeningUnwindTask =
            sdk::cosmwasm_std::from_json(&serialized).expect("the task round-trips");

        assert_eq!(
            serialized,
            sdk::cosmwasm_std::to_json_vec(&restored).expect("a serializable task"),
        );
        assert_eq!(task.baseline, restored.baseline);
    }

    /// A pre-existing balance in a drained currency is not mistaken for an
    /// arrival: the entry baseline is subtracted, so the same balance at the
    /// arrival check leaves nothing received.
    #[test]
    fn pre_existing_balance_is_not_an_arrival() {
        let querier = balances(&[(Lpn::dto().definition().bank_symbol, DOWNPAYMENT + PRINCIPAL)]);
        let task = same_currency_task(&querier);

        assert_eq!(Ok(false), received(&task, &querier));
    }

    /// Same-currency downpayment and principal must BOTH arrive: only the
    /// downpayment landing over the baseline is not enough.
    #[test]
    fn same_currency_requires_both_legs() {
        let baseline = balances(&[]);
        let task = same_currency_task(&baseline);

        let half = balances(&[(Lpn::dto().definition().bank_symbol, DOWNPAYMENT)]);
        assert_eq!(Ok(false), received(&task, &half));

        let full = balances(&[(Lpn::dto().definition().bank_symbol, DOWNPAYMENT + PRINCIPAL)]);
        assert_eq!(Ok(true), received(&task, &full));
    }

    /// Same-currency arrival is measured over the baseline: a pre-existing
    /// balance plus only one leg is still short.
    #[test]
    fn same_currency_arrival_measured_over_baseline() {
        let pre_existing = 500;
        let baseline = balances(&[(Lpn::dto().definition().bank_symbol, pre_existing)]);
        let task = same_currency_task(&baseline);

        let one_leg = balances(&[(
            Lpn::dto().definition().bank_symbol,
            pre_existing + DOWNPAYMENT,
        )]);
        assert_eq!(Ok(false), received(&task, &one_leg));

        let both = balances(&[(
            Lpn::dto().definition().bank_symbol,
            pre_existing + DOWNPAYMENT + PRINCIPAL,
        )]);
        assert_eq!(Ok(true), received(&task, &both));
    }

    /// Distinct currencies each clear independently: both must rise over their
    /// own baseline before the drain completes.
    #[test]
    fn distinct_currencies_each_must_arrive() {
        let baseline = balances(&[]);
        let task = distinct_currency_task(&baseline);

        let only_downpayment =
            balances(&[(PaymentC1::dto().definition().bank_symbol, DOWNPAYMENT)]);
        assert_eq!(Ok(false), received(&task, &only_downpayment));

        let only_principal = balances(&[(Lpn::dto().definition().bank_symbol, PRINCIPAL)]);
        assert_eq!(Ok(false), received(&task, &only_principal));

        let both = balances(&[
            (PaymentC1::dto().definition().bank_symbol, DOWNPAYMENT),
            (Lpn::dto().definition().bank_symbol, PRINCIPAL),
        ]);
        assert_eq!(Ok(true), received(&task, &both));
    }

    fn received(task: &OpeningUnwindTask, querier: &MockQuerier<Empty>) -> Result<bool, String> {
        task.all_received(&Addr::unchecked(LEASE), QuerierWrapper::new(querier))
            .map_err(|err| err.to_string())
    }

    fn same_currency_task(baseline_querier: &MockQuerier<Empty>) -> OpeningUnwindTask {
        // PaymentC2 == Lpn, so the downpayment and the principal share a currency.
        task(Coin::<PaymentC2>::new(DOWNPAYMENT).into(), baseline_querier)
    }

    fn distinct_currency_task(baseline_querier: &MockQuerier<Empty>) -> OpeningUnwindTask {
        // PaymentC1 != Lpn, so the downpayment and the principal are distinct.
        task(Coin::<PaymentC1>::new(DOWNPAYMENT).into(), baseline_querier)
    }

    fn task(
        downpayment: DownpaymentCoin,
        baseline_querier: &MockQuerier<Empty>,
    ) -> OpeningUnwindTask {
        OpeningUnwindTask::enter(
            form(),
            downpayment,
            OpenLoanRespResult {
                principal: Coin::<LpnCurrency>::new(PRINCIPAL).into(),
                annual_interest_rate: Percent100::from_percent(5),
            },
            deps(),
            Addr::unchecked("controller"),
            &Addr::unchecked(LEASE),
            QuerierWrapper::new(baseline_querier),
        )
        .expect("the baseline snapshot succeeds")
    }

    fn balances(holdings: &[(&str, Amount)]) -> MockQuerier<Empty> {
        let coins: Vec<CwCoin> = holdings
            .iter()
            .map(|(denom, amount)| CwCoin::new(*amount, *denom))
            .collect();
        MockQuerier::<Empty>::new(&[(LEASE, &coins)])
    }

    fn deps() -> (
        LppGenericRef<LpnCurrency>,
        OracleRef,
        TimeAlarmsRef,
        LeasesRef,
    ) {
        (
            LppGenericRef::unchecked("lpp"),
            OracleRef::unchecked(Addr::unchecked("oracle")),
            TimeAlarmsRef::unchecked("timealarms"),
            LeasesRef::unchecked(Addr::unchecked("leaser")),
        )
    }

    fn form() -> NewLeaseForm {
        NewLeaseForm {
            customer: Addr::unchecked("customer"),
            currency: currency::dto::<currencies::testing::LeaseC2, _>(),
            max_ltd: None,
            position_spec: PositionSpecDTO::new(
                finance::liability::Liability::new(
                    Percent100::from_percent(65),
                    Percent100::from_percent(70),
                    Percent100::from_percent(73),
                    Percent100::from_percent(75),
                    Percent100::from_percent(78),
                    Percent100::from_percent(80),
                    Duration::from_days(20),
                ),
                Coin::<LpnCurrency>::new(1_000).into(),
                Coin::<LpnCurrency>::new(100).into(),
            ),
            loan: LoanForm {
                lpp: Addr::unchecked("lpp"),
                profit: Addr::unchecked("profit"),
                annual_margin_interest: Percent100::from_permille(31),
                due_period: Duration::from_secs(100),
            },
            reserve: Addr::unchecked("reserve"),
            time_alarms: Addr::unchecked("timealarms"),
            market_price_oracle: Addr::unchecked("oracle"),
        }
    }
}
