use cw_time::IntoInstant;
use dex::{Account, Enterable};
use finance::{coin::Coin, duration::Duration, instant::Instant};
use platform::{
    batch::Batch, message::Response as MessageResponse,
    state_machine::Response as StateMachineResponse,
};
use remote_lease::{
    callback::{RemoteErrorMessage, RemoteLeaseCallback, RemoteOperationOutcome},
    msg::OpenLeaseParams,
    response::{OpenLeaseResponse, RemoteLeaseId, WireOperationResponse},
    stub::{ControllerInnerMessage, Factory},
};
use sdk::cosmwasm_std::{Env, MessageInfo, QuerierWrapper};
use serde::{Deserialize, Serialize};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{
        DownpaymentCoin,
        open::NewLeaseContract,
        query::{StateResponse as QueryStateResponse, opening::OngoingTrx},
    },
    contract::{
        api::Contract,
        cmd::OpenLoanRespResult,
        finalize::LeasesRef,
        state::{Response, State},
    },
    error::{ContractError, ContractResult},
    finance::{LpnCurrency, LppRef, OracleRef, ReserveRef},
};

use super::refund::{OpenFailureRefund, refund_to_open_failed};

#[cfg(all(feature = "internal.test.contract", test))]
mod tests {
    use currencies::testing::{LeaseC2, PaymentC1};
    use cw_time::IntoInstant;
    use dex::{ConnectionParams, Ics20Channel, MaxSlippage};
    use finance::{
        coin::Coin, duration::Duration, instant::Instant, liability::Liability, percent::Percent100,
    };
    use lpp::{msg::LoanResponse, stub::LppRef as LppGenericRef};
    use remote_lease::{
        callback::{RemoteErrorMessage, RemoteLeaseCallback, RemoteOperationOutcome},
        response::{OpenLeaseResponse, RemoteLeaseId, TransferOutResponse, WireOperationResponse},
    };
    use sdk::cosmwasm_std::{
        self, Addr, ContractResult as CwContractResult, Empty, MessageInfo, QuerierResult,
        QuerierWrapper, SystemResult, WasmQuery,
        testing::{self, MockQuerier},
    };
    use timealarms::stub::TimeAlarmsRef;

    use crate::{
        api::{
            authz::{AccessCheck, AccessGranted},
            limits::{MaxSlippages, PositionLimits},
            open::{LoanForm, NewLeaseContract, NewLeaseForm, PositionSpecDTO},
            query::StateResponse as QueryStateResponse,
        },
        contract::{
            api::Contract,
            cmd::OpenLoanRespResult,
            finalize::LeasesRef,
            state::{Response, State},
        },
        error::ContractError,
        finance::{LpnCurrencies, LpnCurrency, OracleRef},
    };

    use super::OpenLease;

    const CONTROLLER: &str = "controller";
    const LEASER: &str = "leaser";
    const LPP: &str = "lpp";
    const RESERVE: &str = "reserve";
    const STRANGER: &str = "stranger";
    const REMOTE_LEASE_ID: &str = "StubPda1111111111111111111111111111";
    const DOWNPAYMENT: u128 = 100;
    const PRINCIPAL: u128 = 500;
    const INTEREST_RATE_PERCENT: u32 = 5;
    const MAX_SLIPPAGE_PERCENT: u32 = 20;
    const INSTANCE_ORDINAL: u16 = 1;

    #[test]
    fn open_lease_ack_starts_the_funding_leg() {
        let response = deliver(open_lease_ack(), CONTROLLER, &authorized_querier())
            .expect("the open-lease ack should start funding");

        assert!(matches!(response.next_state, State::BuyAsset(_)));
    }

    #[test]
    fn unexpected_ok_ack_fails_the_open_with_a_fixed_reason() {
        let response = deliver(unexpected_ok_ack(), CONTROLLER, &authorized_querier())
            .expect("the unexpected ack should fail the open");

        assert_eq!(
            "unexpected operation response",
            open_failed_reason(response)
        );
    }

    #[test]
    fn error_ack_fails_the_open_echoing_the_reason() {
        const REASON: &str = "solana rejected the open";

        let response = deliver(error_ack(REASON), CONTROLLER, &authorized_querier())
            .expect("the error ack should fail the open");

        assert_eq!(REASON, open_failed_reason(response));
    }

    #[test]
    fn timeout_fails_the_open_with_the_timeout_reason() {
        let response = deliver(timeout_ack(), CONTROLLER, &authorized_querier())
            .expect("the timeout should fail the open");

        assert_eq!("timeout", open_failed_reason(response));
    }

    #[test]
    fn callback_from_unauthorized_sender_rejected() {
        assert!(matches!(
            deliver(open_lease_ack(), STRANGER, &rejecting_querier()),
            Err(ContractError::Unauthorized(_))
        ));
    }

    // The authorized ack clears the permission gate and reaches
    // `on_open_lease_ack`, but `open_transport`'s max-slippages query fails, so
    // the error propagates at that `?` instead of the funding leg starting.
    #[test]
    fn open_transport_query_failure_propagates() {
        assert!(matches!(
            deliver(
                open_lease_ack(),
                CONTROLLER,
                &max_slippage_failing_querier()
            ),
            Err(ContractError::PositionLimitsQuery(_))
        ));
    }

    fn deliver(
        callback: RemoteLeaseCallback,
        sender: &str,
        querier: &MockQuerier<Empty>,
    ) -> Result<Response, ContractError> {
        open_lease().on_remote_lease_callback(
            callback,
            info(sender),
            QuerierWrapper::new(querier),
            testing::mock_env(),
        )
    }

    fn open_failed_reason(response: Response) -> String {
        let mock_querier = MockQuerier::<Empty>::default();
        match response.next_state {
            State::OpenFailed(failed) => match failed
                .state(
                    testing::mock_env().block.time.into_instant(),
                    Duration::from_secs(0),
                    QuerierWrapper::new(&mock_querier),
                )
                .expect("the open-failed state query succeeds")
            {
                QueryStateResponse::OpenFailed { reason } => reason.as_str().to_owned(),
                _ => panic!("the open-failed state should report an OpenFailed response"),
            },
            _ => panic!("the callback should land the lease in OpenFailed"),
        }
    }

    fn open_lease() -> OpenLease {
        OpenLease::new(
            new_lease_contract(),
            Coin::<PaymentC1>::new(DOWNPAYMENT).into(),
            OpenLoanRespResult {
                principal: Coin::<LpnCurrency>::new(PRINCIPAL).into(),
                annual_interest_rate: Percent100::from_percent(INTEREST_RATE_PERCENT),
            },
            deps(),
            Instant::from_seconds(1_000_000),
        )
    }

    fn open_lease_ack() -> RemoteLeaseCallback {
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationOk(WireOperationResponse::OpenLease(
                OpenLeaseResponse {
                    remote_lease_id: remote_lease_id(),
                },
            )),
        }
    }

    fn unexpected_ok_ack() -> RemoteLeaseCallback {
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationOk(WireOperationResponse::TransferOut(
                TransferOutResponse {},
            )),
        }
    }

    fn error_ack(reason: &str) -> RemoteLeaseCallback {
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationErr(
                RemoteErrorMessage::new(reason).expect("within the cap"),
            ),
        }
    }

    fn timeout_ack() -> RemoteLeaseCallback {
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationTimeout,
        }
    }

    fn authorized_querier() -> MockQuerier<Empty> {
        let mut mock_querier = MockQuerier::<Empty>::default();
        mock_querier.update_wasm(answer_authorized_queries);
        mock_querier
    }

    fn rejecting_querier() -> MockQuerier<Empty> {
        let mut mock_querier = MockQuerier::<Empty>::default();
        mock_querier.update_wasm(|query| {
            let WasmQuery::Smart { msg, .. } = query else {
                unimplemented!("only smart queries are expected")
            };
            let _: AccessCheck =
                cosmwasm_std::from_json(msg).expect("a remote-lease callback permission query");
            SystemResult::Ok(CwContractResult::Ok(
                cosmwasm_std::to_json_binary(&AccessGranted::No)
                    .expect("the verdict should serialize"),
            ))
        });
        mock_querier
    }

    /// Grants the callback-permission check so the ack reaches
    /// `on_open_lease_ack`, then fails the leaser's max-slippages query so
    /// `open_transport` propagates the error rather than starting the funding
    /// leg.
    fn max_slippage_failing_querier() -> MockQuerier<Empty> {
        let mut mock_querier = MockQuerier::<Empty>::default();
        mock_querier.update_wasm(|query| {
            let WasmQuery::Smart { contract_addr, msg } = query else {
                unimplemented!("only smart queries are expected")
            };
            assert_eq!(LEASER, contract_addr.as_str());
            if cosmwasm_std::from_json::<AccessCheck>(msg).is_ok() {
                SystemResult::Ok(CwContractResult::Ok(
                    cosmwasm_std::to_json_binary(&AccessGranted::Yes)
                        .expect("the verdict should serialize"),
                ))
            } else {
                let _: PositionLimits =
                    cosmwasm_std::from_json(msg).expect("a max-slippages query");
                SystemResult::Ok(CwContractResult::Err(
                    "the leaser is unavailable".to_owned(),
                ))
            }
        });
        mock_querier
    }

    /// Answers every query the authorized open-failure and funding paths make:
    /// the leaser's callback-permission grant and opening slippage bound, the
    /// reserve's Lpn, and the LPP loan. The loan's interest was last paid at the
    /// callback instant, so no interest is due and the reserve cover is skipped.
    fn answer_authorized_queries(query: &WasmQuery) -> QuerierResult {
        let WasmQuery::Smart { contract_addr, msg } = query else {
            unimplemented!("only smart queries are expected")
        };
        let response = match contract_addr.as_str() {
            LEASER => {
                if cosmwasm_std::from_json::<AccessCheck>(msg).is_ok() {
                    cosmwasm_std::to_json_binary(&AccessGranted::Yes)
                } else {
                    let _: PositionLimits =
                        cosmwasm_std::from_json(msg).expect("a max-slippages query");
                    cosmwasm_std::to_json_binary(&max_slippages())
                }
            }
            RESERVE => cosmwasm_std::to_json_binary(&currency::dto::<LpnCurrency, LpnCurrencies>()),
            LPP => cosmwasm_std::to_json_binary(&Some(loan_response())),
            other => unimplemented!("no query is expected against {other}"),
        }
        .expect("the response should serialize");
        SystemResult::Ok(CwContractResult::Ok(response))
    }

    fn max_slippages() -> MaxSlippages {
        MaxSlippages {
            opening: MaxSlippage::unchecked(Percent100::from_percent(MAX_SLIPPAGE_PERCENT)),
            liquidation: MaxSlippage::unchecked(Percent100::from_percent(MAX_SLIPPAGE_PERCENT)),
        }
    }

    fn loan_response() -> LoanResponse<LpnCurrency> {
        LoanResponse {
            principal_due: Coin::new(PRINCIPAL),
            annual_interest_rate: Percent100::from_percent(INTEREST_RATE_PERCENT),
            interest_paid: testing::mock_env().block.time.into_instant(),
        }
    }

    fn new_lease_contract() -> NewLeaseContract {
        NewLeaseContract {
            form: form(),
            dex: connection_params(),
            finalizer: Addr::unchecked("finalizer"),
            remote_lease_controller: Addr::unchecked(CONTROLLER),
            expected_instance_ordinal: INSTANCE_ORDINAL,
        }
    }

    fn form() -> NewLeaseForm {
        NewLeaseForm {
            customer: Addr::unchecked("customer"),
            currency: currency::dto::<LeaseC2, _>(),
            max_ltd: None,
            position_spec: PositionSpecDTO::new(
                liability(),
                Coin::<LpnCurrency>::new(1_000).into(),
                Coin::<LpnCurrency>::new(100).into(),
            ),
            loan: LoanForm {
                lpp: Addr::unchecked(LPP),
                profit: Addr::unchecked("profit"),
                annual_margin_interest: Percent100::from_permille(31),
                due_period: Duration::from_secs(100),
            },
            reserve: Addr::unchecked(RESERVE),
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

    fn deps() -> (
        LppGenericRef<LpnCurrency>,
        OracleRef,
        TimeAlarmsRef,
        LeasesRef,
    ) {
        (
            LppGenericRef::unchecked(LPP),
            OracleRef::unchecked(Addr::unchecked("oracle")),
            TimeAlarmsRef::unchecked("timealarms"),
            LeasesRef::unchecked(Addr::unchecked(LEASER)),
        )
    }

    fn connection_params() -> ConnectionParams {
        ConnectionParams {
            connection_id: "connection-0".to_owned(),
            transfer_channel: Ics20Channel {
                local_endpoint: "channel-0".to_owned(),
                remote_endpoint: "channel-2048".to_owned(),
            },
        }
    }

    fn remote_lease_id() -> RemoteLeaseId {
        RemoteLeaseId::new(REMOTE_LEASE_ID.to_owned()).expect("a base58 sample")
    }

    fn info(sender: &str) -> MessageInfo {
        MessageInfo {
            sender: Addr::unchecked(sender),
            funds: vec![],
        }
    }
}

/// Open-failure reason recorded when the IBC layer reports the OpenLease
/// packet was never acknowledged.
const TIMEOUT_REASON: &str = "timeout";

/// Open-failure reason recorded when the counterparty acks the OpenLease
/// packet with a success response for a different operation. The offending
/// variant is intentionally not echoed into the reason — it is
/// counterparty-controlled and the controller already logs the raw
/// response; keeping a fixed string avoids interpolating unbounded
/// attacker-influenced data into stored state and events.
const UNEXPECTED_OPERATION_REASON: &str = "unexpected operation response";

#[derive(Serialize, Deserialize)]
pub(crate) struct OpenLease {
    new_lease: NewLeaseContract,
    downpayment: DownpaymentCoin,
    loan: OpenLoanRespResult,
    deps: (LppRef, OracleRef, TimeAlarmsRef, LeasesRef),
    start_opening_at: Instant,
}

impl OpenLease {
    pub(super) fn new(
        new_lease: NewLeaseContract,
        downpayment: DownpaymentCoin,
        loan: OpenLoanRespResult,
        deps: (LppRef, OracleRef, TimeAlarmsRef, LeasesRef),
        start_opening_at: Instant,
    ) -> Self {
        Self {
            new_lease,
            downpayment,
            loan,
            deps,
            start_opening_at,
        }
    }

    pub(super) fn enter(&self) -> ContractResult<Batch> {
        OpenLeaseParams::new(
            self.new_lease.expected_instance_ordinal,
            self.downpayment.currency(),
            self.loan.principal.currency().into_super_group(),
            self.new_lease.form.currency.into_super_group(),
        )
        .map_err(ContractError::OpenLeaseParams)
        .and_then(|params| {
            Factory::new(&self.new_lease.remote_lease_controller)
                .open(params, OpenLeaseParams::TIMEOUT, |params, timeout| {
                    ControllerExecuteMsg::OpenLease { params, timeout }
                })
                .map_err(ContractError::from)
        })
    }

    fn authz_callback(
        &self,
        querier: QuerierWrapper<'_>,
        info: &MessageInfo,
    ) -> ContractResult<()> {
        access_control::check(&self.deps.3.remote_lease_callback_permission(querier), info)
            .map_err(Into::into)
    }

    fn on_open_lease_ack(
        self,
        remote_lease_id: RemoteLeaseId,
        querier: QuerierWrapper<'_>,
        env: &Env,
    ) -> ContractResult<Response> {
        let transport = super::buy_asset::open_transport(
            &self.deps.3,
            self.new_lease.remote_lease_controller,
            remote_lease_id,
            querier,
        )?;
        let account = Account::funding(env.contract.address.clone(), self.new_lease.dex);
        let next = super::buy_asset::start(
            self.new_lease.form,
            account,
            self.downpayment,
            self.loan,
            self.deps,
            self.start_opening_at,
            transport,
        )?;
        next.enter(env.block.time.into_instant(), querier)
            .map_err(Into::into)
            .map(|batch| Self::opening_response(next, batch))
    }

    /// Wrap the funding leg's entry batch and its dex state into the lease's
    /// state-machine response.
    fn opening_response(next: super::buy_asset::StartState, batch: Batch) -> Response {
        StateMachineResponse::from(
            MessageResponse::messages_only(batch),
            State::from(super::buy_asset::DexState::from(next)),
        )
    }

    fn on_open_failed(
        self,
        querier: QuerierWrapper<'_>,
        env: &Env,
        reason: RemoteErrorMessage,
    ) -> ContractResult<Response> {
        let Self {
            new_lease,
            downpayment,
            loan,
            deps: (lpp_ref, _oracle, _time_alarms, leases_ref),
            start_opening_at: _,
        } = self;
        let lease_addr = env.contract.address.clone();
        let now = env.block.time.into_instant();
        Coin::<LpnCurrency>::try_from(loan.principal)
            .map_err(ContractError::from)
            .and_then(|principal| {
                ReserveRef::try_new(new_lease.form.reserve.clone(), &querier)
                    .map_err(ContractError::from)
                    .map(|reserve| (principal, reserve))
            })
            .and_then(|(principal, reserve)| {
                refund_to_open_failed(
                    OpenFailureRefund {
                        downpayment,
                        principal,
                        customer: new_lease.form.customer,
                        reserve,
                        lpp_ref,
                        leases_ref,
                        lease_addr,
                        now,
                    },
                    reason,
                    querier,
                )
            })
    }
}

impl Contract for OpenLease {
    fn state(
        self,
        _now: Instant,
        _due_projection: Duration,
        _querier: QuerierWrapper<'_>,
    ) -> ContractResult<QueryStateResponse> {
        Ok(QueryStateResponse::Opening {
            currency: self.new_lease.form.currency,
            downpayment: self.downpayment,
            loan: self.loan.principal,
            loan_interest_rate: self.loan.annual_interest_rate,
            in_progress: OngoingTrx::RequestingOpenLease {},
        })
    }

    fn on_remote_lease_callback(
        self,
        callback: RemoteLeaseCallback,
        info: MessageInfo,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> ContractResult<Response> {
        // OpenLease is a single-shot operation with no duplicate-callback
        // exposure to close, so its callback nonce is not yet correlated; the
        // matching arrives when this flow converts to a remote leg.
        self.authz_callback(querier, &info)
            .and_then(|()| match callback.outcome {
                RemoteOperationOutcome::OperationOk(WireOperationResponse::OpenLease(
                    OpenLeaseResponse { remote_lease_id },
                )) => self.on_open_lease_ack(remote_lease_id, querier, &env),
                RemoteOperationOutcome::OperationOk(_unexpected) => {
                    // A success ack for a non-`OpenLease` operation can only
                    // come from a buggy or hostile counterparty. Returning
                    // `Err` here would revert the controller's
                    // `ibc_packet_ack`, stranding the relayer and freezing the
                    // lease in `OpenLease`. Treat it as an open failure
                    // instead: refund the customer and move to the terminal
                    // `OpenFailed` so the ack commits and operators see a
                    // `wasm-ls-remote-lease-open-failed` event to audit.
                    self.on_open_failed(
                        querier,
                        &env,
                        RemoteErrorMessage::truncated(UNEXPECTED_OPERATION_REASON),
                    )
                }
                RemoteOperationOutcome::OperationErr(reason) => {
                    self.on_open_failed(querier, &env, reason)
                }
                RemoteOperationOutcome::OperationTimeout => self.on_open_failed(
                    querier,
                    &env,
                    RemoteErrorMessage::truncated(TIMEOUT_REASON),
                ),
            })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum ControllerExecuteMsg {
    OpenLease {
        params: OpenLeaseParams,
        timeout: Duration,
    },
}

impl ControllerInnerMessage for ControllerExecuteMsg {}
