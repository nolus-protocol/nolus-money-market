use calculator::Factory as CalculatorFactory;
use currency::{AnyVisitor, CurrencyDTO, CurrencyDef, Group, MemberOf};
use finish::BuyAssetFinish;
use oracle::stub::SwapPath;
use serde::{Deserialize, Serialize};

use dex::MaxSlippage;
use dex::{
    Account, CoinsNb, Connectable, ContractInRemoteSwap, ContractInSwap, Error as DexError,
    FundingClient, RemoteSwapClient, SlippageEscalation, Stage, SwapOutputTask, SwapTask,
    WithCalculator, WithOutputTask,
};
use finance::coin::{Amount, Coin};
use finance::instant::Instant;
use finance::{coin::CoinDTO, duration::Duration};
use platform::batch::Batch;
use platform::ica::HostAccount;
use remote_lease::{
    msg::SwapParams,
    response::{OperationResponse, RemoteLeaseId, SwapResponse},
    stub::{ControllerInnerMessage, Lease as ControllerLease},
};
use sdk::cosmwasm_std::{self, Addr, MessageInfo, QuerierWrapper};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{
        DownpaymentCoin, LeaseAssetCurrencies, LeasePaymentCurrencies,
        open::{NewLeaseContract, NewLeaseForm},
        query::{StateResponse as QueryStateResponse, opening::OngoingTrx},
    },
    contract::{
        cmd::OpenLoanRespResult,
        finalize::LeasesRef,
        state::{
            SwapResult,
            out_task::{OutTaskFactory, WithOutCurrency},
            resp_delivery::ForwardToDexEntry,
        },
    },
    error::ContractResult,
    event::Type,
    finance::{LppRef, OracleRef},
};

mod calculator;
mod finish;

/// A non-`Swap` success acknowledgment can only come from a buggy or
/// hostile counterparty. The fixed reason keeps the unexpected,
/// counterparty-controlled variant out of stored state and events.
const NON_SWAP_RESPONSE: &str = "non-swap operation response";

/// The acknowledged output currency does not belong to the lease asset
/// group, so the response cannot have originated from the scheduled swap.
const OUT_NOT_AN_ASSET: &str = "swapped-out currency is not a lease asset";

const TIMEOUT_RETRY_BUDGET: CoinsNb = 3;

type AssetGroup = LeaseAssetCurrencies;
pub(super) type StartState = dex::StartFundRemoteState<BuyAsset, ForwardToDexEntry>;
pub(in super::super) type DexState = dex::StateFundRemote<BuyAsset, ForwardToDexEntry>;

pub(super) fn start(
    new_lease: NewLeaseContract,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppRef, OracleRef, TimeAlarmsRef, LeasesRef),
    start_opening_at: Instant,
    transport: RemoteSwapTransport,
    lease: Addr,
) -> ContractResult<StartState> {
    let NewLeaseContract { form, dex, .. } = new_lease;
    let spec = BuyAsset::new(
        form,
        Account::funding(lease, dex),
        downpayment,
        loan,
        transport,
        deps,
        start_opening_at,
    );
    dex::start_fund_remote::<_, ForwardToDexEntry>(spec).map_err(Into::into)
}

type BuyAssetStateResponse = <BuyAsset as SwapTask>::StateResponse;

#[derive(Serialize, Deserialize)]
pub struct BuyAsset {
    form: NewLeaseForm,
    dex_account: Account,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    max_slippage: MaxSlippage,
    remote_lease_controller: Addr,
    deps: (LppRef, OracleRef, TimeAlarmsRef, LeasesRef),
    start_opening_at: Instant,
    remote_lease_id: RemoteLeaseId,
    /// The `LeaseAuthority` the funding transfers are addressed to, bridged
    /// from `remote_lease_id` at the start of funding so it can be lent as a
    /// `&HostAccount` to the ICS-20 sender.
    funding_receiver: HostAccount,
}

/// The remote-lease coordinates of the opening: which controller to send the
/// swap legs through, which remote lease they act on, the `LeaseAuthority` the
/// funding transfers are addressed to, and the slippage bound frozen for the
/// whole opening.
pub(super) struct RemoteSwapTransport {
    pub remote_lease_controller: Addr,
    pub remote_lease_id: RemoteLeaseId,
    pub funding_receiver: HostAccount,
    pub max_slippage: MaxSlippage,
}

impl BuyAsset {
    pub(super) fn new(
        form: NewLeaseForm,
        dex_account: Account,
        downpayment: DownpaymentCoin,
        loan: OpenLoanRespResult,
        transport: RemoteSwapTransport,
        deps: (LppRef, OracleRef, TimeAlarmsRef, LeasesRef),
        start_opening_at: Instant,
    ) -> Self {
        let RemoteSwapTransport {
            remote_lease_controller,
            remote_lease_id,
            funding_receiver,
            max_slippage,
        } = transport;
        Self {
            form,
            dex_account,
            downpayment,
            loan,
            max_slippage,
            remote_lease_controller,
            deps,
            start_opening_at,
            remote_lease_id,
            funding_receiver,
        }
    }

    fn state<InP>(self, in_progress_fn: InP) -> BuyAssetStateResponse
    where
        InP: FnOnce(String) -> OngoingTrx,
    {
        Ok(QueryStateResponse::Opening {
            currency: self.form.currency,
            downpayment: self.downpayment,
            loan: self.loan.principal,
            loan_interest_rate: self.loan.annual_interest_rate,
            in_progress: in_progress_fn(self.funding_receiver.into()),
        })
    }
}

impl SwapTask for BuyAsset {
    type InG = LeasePaymentCurrencies;
    type OutG = AssetGroup;
    type Label = Type;
    type StateResponse = ContractResult<QueryStateResponse>;
    type Result = SwapResult;

    fn label(&self) -> Self::Label {
        Type::OpeningSwap
    }

    fn dex_account(&self) -> &Account {
        &self.dex_account
    }

    fn oracle(&self) -> &impl SwapPath<<Self::InG as Group>::TopG> {
        &self.deps.1
    }

    fn time_alarm(&self) -> &TimeAlarmsRef {
        &self.deps.2
    }

    fn authz_remote_callback(
        &self,
        querier: QuerierWrapper<'_>,
        info: &MessageInfo,
    ) -> dex::DexResult<()> {
        access_control::check(&self.deps.3.remote_lease_callback_permission(querier), info)
            .map_err(DexError::Unauthorized)
    }

    fn authz_anomaly_resolution(
        &self,
        querier: QuerierWrapper<'_>,
        info: &MessageInfo,
    ) -> dex::DexResult<()> {
        access_control::check(&self.deps.3.anomaly_resolution_permission(querier), info)
            .map_err(DexError::Unauthorized)
    }

    fn timeout_retry_budget(&self) -> CoinsNb {
        TIMEOUT_RETRY_BUDGET
    }

    /// The opening swap re-emits a timed-out leg verbatim rather than parking
    /// it - it must make forward progress to open. An explicit error still
    /// parks at the slippage-anomaly terminal (see `RemoteSwap::on_remote_error`);
    /// this governs only the timeout path.
    fn slippage_escalation(&self) -> SlippageEscalation {
        SlippageEscalation::ReEmit
    }

    fn coins(&self) -> impl IntoIterator<Item = CoinDTO<Self::InG>> {
        [self.downpayment, self.loan.principal.into_super_group()].into_iter()
    }

    fn with_slippage_calc<WithCalc>(&self, with_calc: WithCalc) -> WithCalc::Output
    where
        WithCalc: WithCalculator<Self>,
    {
        self.form
            .currency
            .into_super_group()
            .into_currency_type(CalculatorFactory::new(
                with_calc,
                self.max_slippage,
                &self.deps.1,
            ))
    }

    fn into_output_task<Cmd>(self, cmd: Cmd) -> Cmd::Output
    where
        Cmd: WithOutputTask<Self>,
    {
        struct OutputTaskFactory {}
        impl OutTaskFactory<BuyAsset> for OutputTaskFactory {
            fn new_task<OutC>(swap_task: BuyAsset) -> impl SwapOutputTask<BuyAsset, OutC = OutC>
            where
                OutC: CurrencyDef,
                OutC::Group: MemberOf<<BuyAsset as SwapTask>::OutG>
                    + MemberOf<<<BuyAsset as SwapTask>::InG as Group>::TopG>,
            {
                BuyAssetFinish::<_, OutC>::from(swap_task)
            }
        }
        self.form
            .currency
            .into_super_group()
            .into_currency_type(WithOutCurrency::<_, OutputTaskFactory, _>::from(self, cmd))
    }
}

impl RemoteSwapClient for BuyAsset {
    fn schedule_swap(
        &self,
        coin_in: &CoinDTO<Self::InG>,
        min_out: &CoinDTO<Self::OutG>,
        nonce: u64,
    ) -> dex::DexResult<Batch> {
        SwapParams::new(*coin_in, min_out.into_super_group())
            .map_err(DexError::remote_swap_client)
            .and_then(|params| {
                ControllerLease::new(&self.remote_lease_controller)
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

    fn decode_response(&self, payload: &[u8]) -> dex::DexResult<CoinDTO<Self::OutG>> {
        cosmwasm_std::from_json::<OperationResponse>(payload)
            .map_err(DexError::remote_swap_client)
            .and_then(|response| match response {
                OperationResponse::Swap(SwapResponse { amount_out }) => {
                    into_asset_group(&amount_out)
                }
                OperationResponse::OpenLease(_)
                | OperationResponse::CloseLease(_)
                | OperationResponse::TransferOut(_) => {
                    Err(DexError::unexpected_response_variant(NON_SWAP_RESPONSE))
                }
            })
    }
}

impl FundingClient for BuyAsset {
    fn funding_sender(&self) -> &Addr {
        self.dex_account.owner()
    }

    fn funding_receiver(&self) -> &HostAccount {
        &self.funding_receiver
    }

    fn transfer_channel(&self) -> &str {
        self.dex_account
            .dex()
            .transfer_channel
            .local_endpoint
            .as_str()
    }
}

impl ContractInSwap for BuyAsset {
    type StateResponse = <Self as SwapTask>::StateResponse;

    fn state(
        self,
        in_progress: Stage,
        _now: Instant,
        _due_projection: Duration,
        _querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        match in_progress {
            Stage::TransferOut => self.state(|receiver| OngoingTrx::Funding { receiver }),
            Stage::Swap => unimplemented!("the opening swap runs over the remote-lease transport"),
            Stage::TransferInInit => unimplemented!(),
            Stage::TransferInFinish => unimplemented!(),
        }
    }
}

impl ContractInRemoteSwap for BuyAsset {
    type StateResponse = <Self as SwapTask>::StateResponse;

    fn state(
        self,
        acks_left: CoinsNb,
        _now: Instant,
        _due_projection: Duration,
        _querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        self.state(|_ica_account| OngoingTrx::BuyAsset { acks_left })
    }

    fn anomaly_response(
        self,
        _acks_left: CoinsNb,
        _now: Instant,
        _due_projection: Duration,
        _querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        self.state(|_ica_account| OngoingTrx::SlippageProtectionActivated)
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

fn into_asset_group(
    amount_out: &CoinDTO<LeasePaymentCurrencies>,
) -> dex::DexResult<CoinDTO<AssetGroup>> {
    struct AsAssetCoin {
        amount: Amount,
    }
    impl AnyVisitor<AssetGroup> for AsAssetCoin {
        type Outcome = CoinDTO<AssetGroup>;

        fn on<C>(self, _def: &CurrencyDTO<C::Group>) -> Self::Outcome
        where
            C: CurrencyDef,
            C::Group: MemberOf<AssetGroup> + MemberOf<<AssetGroup as Group>::TopG>,
        {
            Coin::<C>::new(self.amount).into()
        }
    }

    amount_out
        .currency()
        .may_into_currency_type::<AssetGroup, _>(AsAssetCoin {
            amount: amount_out.amount(),
        })
        .map_err(|_not_an_asset| DexError::unexpected_response_variant(OUT_NOT_AN_ASSET))
}

#[cfg(all(feature = "internal.test.contract", test))]
mod tests {
    use currencies::{
        Lpn,
        testing::{LeaseC2, PaymentC1},
    };
    use dex::{
        Account, ConnectionParams, Ics20Channel, MaxSlippage, RemoteSwapClient, SlippageCalculator,
        SwapTask, WithCalculator,
    };
    use finance::{
        coin::{Coin, CoinDTO},
        duration::Duration,
        instant::Instant,
        liability::Liability,
        percent::Percent100,
        price::base::BasePrice,
    };
    use lpp::stub::LppRef as LppGenericRef;
    use platform::ica::HostAccount;
    use remote_lease::{
        msg::SwapParams,
        response::{
            CloseLeaseResponse, OpenLeaseResponse, OperationResponse, RemoteLeaseId, SwapResponse,
            TransferOutResponse,
        },
    };
    use sdk::cosmwasm_std::{
        self, Addr, ContractResult as CwContractResult, QuerierWrapper, SystemResult, WasmQuery,
        testing::MockQuerier,
    };
    use serde::Deserialize;
    use timealarms::stub::TimeAlarmsRef;

    use crate::{
        api::{
            LeaseAssetCurrencies, LeasePaymentCurrencies,
            open::{LoanForm, NewLeaseForm, PositionSpecDTO},
        },
        contract::{cmd::OpenLoanRespResult, finalize::LeasesRef},
        finance::{LpnCurrencies, LpnCurrency, OracleRef},
    };

    use super::{BuyAsset, RemoteSwapTransport};

    const MAX_SLIPPAGE_PERCENT: u32 = 20;
    const DOWNPAYMENT_IN: u128 = 100;
    const PAYMENT_PRICE_IN_LPN: u128 = 2;
    const ASSET_PRICE_IN_LPN: u128 = 4;

    #[test]
    fn swap_msg_shape_matches_the_controller_wire() {
        const SWAP_NONCE: u64 = 7;
        let params = swap_params();
        let msg = super::ControllerExecuteMsg::Swap {
            params: params.clone(),
            timeout: SwapParams::TIMEOUT,
            nonce: SWAP_NONCE,
        };

        let expected = format!(
            r#"{{"swap":{{"params":{},"timeout":{},"nonce":{}}}}}"#,
            cosmwasm_std::to_json_string(&params).expect("the params should serialize"),
            cosmwasm_std::to_json_string(&SwapParams::TIMEOUT)
                .expect("the timeout should serialize"),
            SWAP_NONCE,
        );
        assert_eq!(
            expected,
            cosmwasm_std::to_json_string(&msg).expect("the message should serialize")
        );
    }

    #[test]
    fn decode_accepts_a_swap_response() {
        let amount_out: CoinDTO<LeasePaymentCurrencies> = Coin::<LeaseC2>::new(1000).into();
        let payload = OperationResponse::Swap(SwapResponse { amount_out });

        assert_eq!(
            Ok(Coin::<LeaseC2>::new(1000).into()),
            decode(&payload).map_err(|err| err.to_string())
        );
    }

    #[test]
    fn decode_rejects_non_swap_responses() {
        let unexpected = [
            OperationResponse::OpenLease(OpenLeaseResponse {
                remote_lease_id: remote_lease_id(),
            }),
            OperationResponse::CloseLease(CloseLeaseResponse {}),
            OperationResponse::TransferOut(TransferOutResponse {}),
        ];
        unexpected.into_iter().for_each(|payload| {
            assert!(
                decode(&payload)
                    .expect_err("a non-swap response should be rejected")
                    .to_string()
                    .contains(super::NON_SWAP_RESPONSE)
            );
        });
    }

    #[test]
    fn decode_rejects_a_non_asset_out_currency() {
        let amount_out: CoinDTO<LeasePaymentCurrencies> = Coin::<Lpn>::new(1000).into();
        let payload = OperationResponse::Swap(SwapResponse { amount_out });

        assert!(
            decode(&payload)
                .expect_err("a non-asset output should be rejected")
                .to_string()
                .contains(super::OUT_NOT_AN_ASSET)
        );
    }

    #[test]
    fn min_out_equals_the_slippage_bounded_quote() {
        let mut mock_querier = MockQuerier::default();
        mock_querier.update_wasm(oracle_prices);
        let querier = QuerierWrapper::new(&mock_querier);

        let min_out = spec().with_slippage_calc(ProbeMinOut {
            coin_in: Coin::<PaymentC1>::new(DOWNPAYMENT_IN).into(),
            querier,
        });

        // 100 PaymentC1 = 200 LPN = 50 LeaseC2; 20% slippage => 40
        let quote =
            Coin::<LeaseC2>::new(DOWNPAYMENT_IN * PAYMENT_PRICE_IN_LPN / ASSET_PRICE_IN_LPN);
        let expected: CoinDTO<LeaseAssetCurrencies> =
            MaxSlippage::unchecked(Percent100::from_percent(MAX_SLIPPAGE_PERCENT))
                .min_out(quote)
                .into();
        assert_eq!(Ok(expected), min_out);
    }

    struct ProbeMinOut<'querier, Task>
    where
        Task: SwapTask,
    {
        coin_in: CoinDTO<Task::InG>,
        querier: QuerierWrapper<'querier>,
    }

    impl<Task> WithCalculator<Task> for ProbeMinOut<'_, Task>
    where
        Task: SwapTask,
    {
        type Output = Result<CoinDTO<Task::OutG>, String>;

        fn on<CalculatorT>(self, calculator: &CalculatorT) -> Self::Output
        where
            CalculatorT: SlippageCalculator<Task::InG>,
            <<CalculatorT as SlippageCalculator<Task::InG>>::OutC as currency::CurrencyDef>::Group:
                currency::MemberOf<Task::OutG>
                    + currency::MemberOf<<Task::InG as currency::Group>::TopG>,
        {
            calculator
                .min_output(&self.coin_in, self.querier)
                .map(Into::into)
                .map_err(|err| err.to_string())
        }
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "snake_case")]
    enum OracleQuery {
        BasePrice {
            currency: currency::CurrencyDTO<LeasePaymentCurrencies>,
        },
    }

    fn oracle_prices(query: &WasmQuery) -> sdk::cosmwasm_std::QuerierResult {
        let WasmQuery::Smart { msg, .. } = query else {
            unimplemented!("only smart queries are expected")
        };
        let OracleQuery::BasePrice { currency } =
            cosmwasm_std::from_json(msg).expect("a base-price query");
        let response = if currency == currency::dto::<PaymentC1, LeasePaymentCurrencies>() {
            price_in_lpn::<PaymentC1>(PAYMENT_PRICE_IN_LPN)
        } else {
            assert_eq!(currency::dto::<LeaseC2, LeasePaymentCurrencies>(), currency);
            price_in_lpn::<LeaseC2>(ASSET_PRICE_IN_LPN)
        };
        SystemResult::Ok(CwContractResult::Ok(response))
    }

    fn price_in_lpn<C>(quote_amount: u128) -> sdk::cosmwasm_std::Binary
    where
        C: currency::CurrencyDef,
        C::Group: currency::MemberOf<LeasePaymentCurrencies>,
    {
        let price = BasePrice::<LeasePaymentCurrencies, LpnCurrency, LpnCurrencies>::new(
            Coin::<C>::new(1).into(),
            Coin::<LpnCurrency>::new(quote_amount),
        );
        cosmwasm_std::to_json_binary(&price).expect("the price should serialize")
    }

    fn decode(payload: &OperationResponse) -> Result<CoinDTO<LeaseAssetCurrencies>, dex::Error> {
        spec().decode_response(encode(payload).as_slice())
    }

    fn encode(payload: &OperationResponse) -> Vec<u8> {
        cosmwasm_std::to_json_vec(payload).expect("the payload should serialize")
    }

    fn swap_params() -> SwapParams {
        SwapParams::new(
            Coin::<PaymentC1>::new(1000).into(),
            Coin::<LeaseC2>::new(42).into(),
        )
        .expect("distinct non-zero amounts")
    }

    fn remote_lease_id() -> RemoteLeaseId {
        RemoteLeaseId::new("StubPda1111111111111111111111111111".to_owned())
            .expect("a base58 sample")
    }

    fn spec() -> BuyAsset {
        BuyAsset::new(
            form(),
            account(),
            Coin::<PaymentC1>::new(DOWNPAYMENT_IN).into(),
            OpenLoanRespResult {
                principal: Coin::<LpnCurrency>::new(500).into(),
                annual_interest_rate: Percent100::from_percent(5),
            },
            RemoteSwapTransport {
                remote_lease_controller: Addr::unchecked("controller"),
                remote_lease_id: remote_lease_id(),
                funding_receiver: funding_receiver(),
                max_slippage: MaxSlippage::unchecked(Percent100::from_percent(
                    MAX_SLIPPAGE_PERCENT,
                )),
            },
            (
                LppGenericRef::unchecked("lpp"),
                OracleRef::unchecked(Addr::unchecked("oracle")),
                TimeAlarmsRef::unchecked("timealarms"),
                LeasesRef::unchecked(Addr::unchecked("leaser")),
            ),
            Instant::from_seconds(1_000_000),
        )
    }

    fn funding_receiver() -> HostAccount {
        remote_lease_id()
            .as_str()
            .to_owned()
            .try_into()
            .expect("a valid host account")
    }

    fn form() -> NewLeaseForm {
        NewLeaseForm {
            customer: Addr::unchecked("customer"),
            currency: currency::dto::<LeaseC2, _>(),
            max_ltd: None,
            position_spec: PositionSpecDTO::new(
                liability(),
                Coin::<LpnCurrency>::new(1000).into(),
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

    fn liability() -> Liability {
        Liability::new(
            Percent100::from_percent(65),
            Percent100::from_percent(70),
            Percent100::from_percent(73),
            Percent100::from_percent(75),
            Percent100::from_percent(78),
            Percent100::from_percent(80),
            Duration::from_days(20),
        )
    }

    fn account() -> Account {
        Account::unchecked(
            Addr::unchecked("lease"),
            "ica0"
                .to_owned()
                .try_into()
                .expect("a non-empty host account"),
            ConnectionParams {
                connection_id: "connection-0".to_owned(),
                transfer_channel: Ics20Channel {
                    local_endpoint: "channel-0".to_owned(),
                    remote_endpoint: "channel-2048".to_owned(),
                },
            },
        )
    }
}
