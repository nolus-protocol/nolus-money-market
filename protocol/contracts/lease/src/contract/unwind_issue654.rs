//! One-shot administrative unwind of the single `OSMOSIS-OSMOSIS-USDC_AXELAR`
//! lease left frozen in `BuyAsset::TransferOut` since 2024-03-05 (issue #654).
//!
//! The lease never acquired its asset, so its own account still holds the LPN
//! principal and the downpayment. This migrate repays the LPP loan — the reserve
//! covers the accrued interest the lease never set aside the funds for — refunds
//! the downpayment to the customer, finalizes the lease at the leaser, and parks
//! the contract in [`super::state::closed`].
//!
//! It ships only on the `fix/issue654-unwind-axelar-usdc` hotfix branch off
//! `v0.8.24` and never merges back to `main`. The migrate refuses to run unless
//! the stored state matches the audited production snapshot byte for byte, so it
//! can only ever act on the one contract it was crafted for.

use serde::Deserialize;

use currency::{CurrencyDef, Group, MemberOf};
use finance::{
    coin::{Coin, CoinDTO, WithCoin},
    interest,
    percent::Percent100,
    period::Period,
};
use lpp::stub::{
    LppBatch, LppRef as LppGenericRef,
    loan::{LppLoan, WithLppLoan},
};
use platform::{
    bank::{FixedAddressSender, LazySenderStub},
    batch::{Batch, Emit, Emitter},
    contract,
    message::Response as MessageResponse,
    response,
};
use reserve::stub::{Ref as ReserveGenericRef, Reserve};
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{self, Addr, DepsMut, Env, QuerierWrapper, Storage, Timestamp},
};

use crate::{
    api::LeasePaymentCurrencies,
    error::{ContractError, ContractResult},
    event::Type,
    finance::{LpnCoin, LpnCurrency},
};

use super::{finalize::LeasesRef, state};

type LppRef = LppGenericRef<LpnCurrency>;
type ReserveRef = ReserveGenericRef<LpnCurrency>;

const STATE_KEY: &[u8] = b"state";

/// The exact bytes stored under [`STATE_KEY`] by the frozen lease on pirin-1,
/// captured from an archive node. The migrate proceeds only on a byte-for-byte
/// match; the operator runbook records how to re-verify it against the chain.
const AUDITED_STATE: &[u8] = include_bytes!("issue654_state.json");

const MARGIN_OVERFLOW: &str = "issue #654 margin interest";
const LPP_OVERFLOW: &str = "issue #654 LPP interest";

pub(super) fn unwind(deps: DepsMut<'_>, env: Env) -> ContractResult<CwResponse> {
    guard(deps.storage)
        .and_then(|()| parse_spec())
        .and_then(|spec| settle(&spec, &env.contract.address, &env.block.time, deps.querier))
        .and_then(|unwound| {
            state::save(deps.storage, &state::closed())
                .map(|()| response::response_only_messages(unwound))
        })
}

fn guard(storage: &dyn Storage) -> ContractResult<()> {
    if storage.get(STATE_KEY).as_deref() == Some(AUDITED_STATE) {
        Ok(())
    } else {
        Err(ContractError::Issue654StateMismatch())
    }
}

// Deserializes the audited constant, not live storage: [`guard`] runs first in
// [`unwind`] and proves the stored bytes equal `AUDITED_STATE`, so the two are
// interchangeable here and the parsed values are fixed and audited. The lenient
// mirror structs (no `deny_unknown_fields`) are safe only under that ordering —
// do not repoint this at live storage without restoring strict validation.
fn parse_spec() -> ContractResult<Spec> {
    cosmwasm_std::from_json::<Stored>(AUDITED_STATE)
        .map(|stored| stored.buy_asset.transfer_out.spec)
        .map_err(Into::into)
}

fn settle(
    spec: &Spec,
    lease: &Addr,
    now: &Timestamp,
    querier: QuerierWrapper<'_>,
) -> ContractResult<MessageResponse> {
    repay_loan(&spec.form.loan.lpp, lease, now, querier).and_then(|repaid| {
        margin_interest(
            spec.form.loan.annual_margin_interest,
            repaid.principal,
            &spec.start_opening_at,
            now,
        )
        .and_then(|margin| assemble(spec, repaid, margin, querier))
    })
}

fn assemble(
    spec: &Spec,
    repaid: RepayOutcome,
    margin: LpnCoin,
    querier: QuerierWrapper<'_>,
) -> ContractResult<MessageResponse> {
    cover_gap(&spec.form.reserve, repaid.lpp_interest + margin, querier).and_then(|reserve| {
        finalize(&spec.deps.3.addr, spec.form.customer.clone(), querier).map(|finalized| {
            let messages = reserve
                .merge(repaid.messages)
                .merge(send(spec.form.loan.profit.clone(), margin))
                .merge(refund_downpayment(&spec.form.customer, &spec.downpayment))
                .merge(finalized);
            MessageResponse::messages_with_event(messages, unwind_event(spec))
        })
    })
}

fn repay_loan(
    lpp: &Addr,
    lease: &Addr,
    now: &Timestamp,
    querier: QuerierWrapper<'_>,
) -> ContractResult<RepayOutcome> {
    LppRef::try_new(lpp.clone(), querier)
        .map_err(ContractError::LppStubError)
        .and_then(|lpp_ref| lpp_ref.execute_loan(RepayInFull { now: *now }, lease.clone(), querier))
}

fn margin_interest(
    rate: Percent100,
    principal: LpnCoin,
    start: &Timestamp,
    now: &Timestamp,
) -> ContractResult<LpnCoin> {
    interest::interest(rate, principal, Period::from_till(*start, now).length())
        .ok_or(ContractError::Overflow(MARGIN_OVERFLOW))
}

fn cover_gap(reserve: &Addr, gap: LpnCoin, querier: QuerierWrapper<'_>) -> ContractResult<Batch> {
    ReserveRef::try_new(reserve.clone(), &querier)
        .map_err(ContractError::from)
        .and_then(|reserve_ref| {
            let mut reserve_stub = reserve_ref.into_reserve();
            reserve_stub.cover_liquidation_losses(gap);
            reserve_stub.try_into().map_err(ContractError::from)
        })
}

fn finalize(leaser: &Addr, customer: Addr, querier: QuerierWrapper<'_>) -> ContractResult<Batch> {
    LeasesRef::try_new(leaser.clone(), &contract::validator(querier))
        .and_then(|leaser_ref| leaser_ref.finalize_lease(customer))
}

fn refund_downpayment(customer: &Addr, downpayment: &CoinDTO<LeasePaymentCurrencies>) -> Batch {
    downpayment.with_coin(SendToCustomer {
        customer: customer.clone(),
    })
}

fn send<C>(to: Addr, amount: Coin<C>) -> Batch
where
    C: CurrencyDef,
{
    let mut sender = LazySenderStub::new(to);
    sender.send(amount);
    sender.into()
}

fn unwind_event(spec: &Spec) -> Emitter {
    Emitter::of_type(Type::Closed)
        .emit("id", "issue-654-unwind")
        .emit("customer", spec.form.customer.as_str())
}

struct RepayInFull {
    now: Timestamp,
}

struct RepayOutcome {
    principal: LpnCoin,
    lpp_interest: LpnCoin,
    messages: Batch,
}

impl WithLppLoan<LpnCurrency> for RepayInFull {
    type Output = RepayOutcome;

    type Error = ContractError;

    fn exec<LoanStub>(self, mut loan: LoanStub) -> Result<Self::Output, Self::Error>
    where
        LoanStub: LppLoan<LpnCurrency>,
    {
        let principal = loan.principal_due();
        let lpp_interest = loan
            .interest_due(&self.now)
            .ok_or(ContractError::Overflow(LPP_OVERFLOW))?;
        loan.repay(&self.now, principal + lpp_interest)
            .ok_or(ContractError::Overflow(LPP_OVERFLOW))?;
        let batch: LppBatch<LppRef> = loan.try_into().map_err(ContractError::from)?;
        Ok(RepayOutcome {
            principal,
            lpp_interest,
            messages: batch.batch,
        })
    }
}

struct SendToCustomer {
    customer: Addr,
}

impl WithCoin<LeasePaymentCurrencies> for SendToCustomer {
    type Outcome = Batch;

    fn on<C>(self, coin: Coin<C>) -> Self::Outcome
    where
        C: CurrencyDef,
        C::Group:
            MemberOf<LeasePaymentCurrencies> + MemberOf<<LeasePaymentCurrencies as Group>::TopG>,
    {
        send(self.customer, coin)
    }
}

#[derive(Deserialize)]
struct Stored {
    #[serde(rename = "BuyAsset")]
    buy_asset: StateNode,
}

#[derive(Deserialize)]
struct StateNode {
    #[serde(rename = "TransferOut")]
    transfer_out: TransferOutNode,
}

#[derive(Deserialize)]
struct TransferOutNode {
    spec: Spec,
}

#[derive(Deserialize)]
struct Spec {
    form: Form,
    downpayment: CoinDTO<LeasePaymentCurrencies>,
    deps: (RefAddr, RefAddr, RefAddr, RefAddr),
    start_opening_at: Timestamp,
}

#[derive(Deserialize)]
struct Form {
    customer: Addr,
    loan: LoanForm,
    reserve: Addr,
}

#[derive(Deserialize)]
struct LoanForm {
    lpp: Addr,
    profit: Addr,
    annual_margin_interest: Percent100,
}

#[derive(Deserialize)]
struct RefAddr {
    addr: Addr,
}

#[cfg(all(feature = "internal.test.contract", test))]
mod test {
    use currencies::{Lpn, Lpns};
    use finance::{
        coin::{Amount, Coin},
        duration::Duration,
        percent::Percent100,
    };
    use lpp::{
        loan::Loan,
        msg::{ExecuteMsg as LppExecuteMsg, LoanResponse, QueryMsg as LppQueryMsg},
    };
    use platform::response;
    use reserve::api::ExecuteMsg as ReserveExecuteMsg;
    use sdk::{
        cosmwasm_ext::CosmosMsg,
        cosmwasm_std::{
            self, Addr, BankMsg, Binary, ContractResult, QuerierWrapper, StdResult, Storage,
            SystemResult, Timestamp, Uint256, WasmMsg, WasmQuery, testing::MockQuerier,
            testing::MockStorage, to_json_binary,
        },
    };

    use crate::{api::FinalizerExecuteMsg, error::ContractError};

    use super::{AUDITED_STATE, STATE_KEY, Spec, Stored, guard, settle};

    // Same nesting and field layout as the production state captured in
    // `issue654_state.json`, but with test-group currency tickers — only the
    // `downpayment` ticker is type-checked by the mirror, the rest are skipped.
    const LEGACY_STATE: &str = r#"{"BuyAsset":{"TransferOut":{"spec":{"form":{"customer":"the-customer","currency":"LC1","max_ltd":1500,"position_spec":{"liability":{"initial":600,"healthy":830,"first_liq_warn":850,"second_liq_warn":865,"third_liq_warn":880,"max":900,"recalc_time":432000000000000},"min_asset":{"amount":"15000000","ticker":"LPN"},"min_transaction":{"amount":"10000","ticker":"LPN"}},"loan":{"lpp":"the-lpp","profit":"the-profit","annual_margin_interest":40,"due_period":1209600000000000},"reserve":"the-reserve","time_alarms":"the-timealarms","market_price_oracle":"the-oracle"},"dex_account":{"owner":"the-lease","host":"the-host","dex":{"connection_id":"connection-0","transfer_channel":{"local_endpoint":"channel-0","remote_endpoint":"channel-783"}}},"downpayment":{"amount":"220741070","ticker":"LPN"},"loan":{"principal":{"amount":"203510730","ticker":"LPN"},"annual_interest_rate":146},"deps":[{"addr":"the-lpp"},{"addr":"the-oracle","base_currency":"LPN"},{"addr":"the-timealarms"},{"addr":"the-leaser"}],"start_opening_at":"1709669745935648283"},"coin_index":0,"last_coin_index":1}}}"#;

    #[test]
    fn parse_extracts_unwind_targets() {
        let spec = cosmwasm_std::from_json::<Stored>(LEGACY_STATE.as_bytes())
            .expect("the legacy state to deserialize")
            .buy_asset
            .transfer_out
            .spec;

        assert_eq!("the-customer", spec.form.customer.as_str());
        assert_eq!("the-lpp", spec.form.loan.lpp.as_str());
        assert_eq!("the-profit", spec.form.loan.profit.as_str());
        assert_eq!("the-reserve", spec.form.reserve.as_str());
        assert_eq!("the-leaser", spec.deps.3.addr.as_str());
        assert_eq!(
            Percent100::from_permille(40),
            spec.form.loan.annual_margin_interest
        );
        assert_eq!(220_741_070, spec.downpayment.amount());
        assert_eq!(1_709_669_745_935_648_283, spec.start_opening_at.nanos());
    }

    // The lease never acquired its asset, so the LPP loan principal equals what the
    // lease holds. With a round principal and exactly one year of accrual the
    // amounts are exact integers: lpp interest = 14.6% = 146, margin = 4.0% = 40.
    #[test]
    fn settle_emits_full_unwind_batch() {
        const PRINCIPAL: Amount = 1000;
        const LPP_INTEREST: Amount = 146;
        const MARGIN: Amount = 40;
        const GAP: Amount = LPP_INTEREST + MARGIN; // reserve covers the interest the lease lacks
        const REPAYMENT: Amount = PRINCIPAL + LPP_INTEREST; // funds attached to RepayLoan
        const DOWNPAYMENT: Amount = 220_741_070; // refunded whole to the customer
        // matches LEGACY_STATE.start_opening_at, so margin accrues over exactly a year
        const START: u64 = 1_709_669_745_935_648_283;

        let spec = cosmwasm_std::from_json::<Stored>(LEGACY_STATE.as_bytes())
            .expect("the legacy state to deserialize")
            .buy_asset
            .transfer_out
            .spec;
        let start = Timestamp::from_nanos(START);
        let now = start + Duration::YEAR;
        let querier = unwind_querier(&spec, start);

        let resp = response::response_only_messages(
            settle(
                &spec,
                &Addr::unchecked("the-lease"),
                &now,
                QuerierWrapper::new(&querier),
            )
            .expect("the unwind batch to assemble"),
        );

        assert_eq!(5, resp.messages.len());
        assert_cover(&resp.messages[0].msg, &spec.form.reserve, GAP);
        assert_repay(&resp.messages[1].msg, &spec.form.loan.lpp, REPAYMENT);
        assert_bank(&resp.messages[2].msg, &spec.form.loan.profit, MARGIN);
        assert_bank(&resp.messages[3].msg, &spec.form.customer, DOWNPAYMENT);
        assert_finalize(
            &resp.messages[4].msg,
            &spec.deps.3.addr,
            &spec.form.customer,
        );
    }

    fn unwind_querier(spec: &Spec, loan_start: Timestamp) -> MockQuerier {
        const CONTRACT_INFO: &str = r#"{"code_id":1,"creator":"creator","admin":null,"pinned":false,"ibc_port":null,"ibc2_port":null}"#;

        let lpp = spec.form.loan.lpp.clone();
        let reserve = spec.form.reserve.clone();
        let leaser = spec.deps.3.addr.clone();
        let mut querier = MockQuerier::default();
        querier.update_wasm(move |request| {
            let answer: StdResult<Binary> = match request {
                WasmQuery::Smart { contract_addr, msg } if *contract_addr == lpp.as_str() => {
                    match cosmwasm_std::from_json::<LppQueryMsg<Lpns>>(msg).expect("an Lpp query") {
                        LppQueryMsg::Lpn() => to_json_binary(&currency::dto::<Lpn, Lpns>()),
                        LppQueryMsg::Loan { .. } => to_json_binary(&Some(loan(loan_start))),
                        _ => panic!("unexpected Lpp query"),
                    }
                }
                WasmQuery::Smart { contract_addr, .. } if *contract_addr == reserve.as_str() => {
                    to_json_binary(&currency::dto::<Lpn, Lpns>())
                }
                WasmQuery::ContractInfo { contract_addr } if *contract_addr == leaser.as_str() => {
                    Ok(Binary::new(CONTRACT_INFO.as_bytes().to_vec()))
                }
                other => panic!("unexpected query: {other:?}"),
            };
            SystemResult::Ok(ContractResult::Ok(answer.expect("a serializable answer")))
        });
        querier
    }

    fn loan(start: Timestamp) -> LoanResponse<Lpn> {
        Loan {
            principal_due: Coin::new(1000),
            annual_interest_rate: Percent100::from_permille(146),
            interest_paid: start,
        }
    }

    fn assert_cover(msg: &CosmosMsg, reserve: &Addr, amount: Amount) {
        let (contract_addr, exec, funds) = as_execute::<ReserveExecuteMsg>(msg);
        assert_eq!(reserve.as_str(), contract_addr);
        assert!(funds.is_empty());
        match exec {
            ReserveExecuteMsg::CoverLiquidationLosses(coin) => assert_eq!(amount, coin.amount()),
            other => panic!("expected CoverLiquidationLosses, got {other:?}"),
        }
    }

    fn assert_repay(msg: &CosmosMsg, lpp: &Addr, amount: Amount) {
        let (contract_addr, exec, funds) = as_execute::<LppExecuteMsg<Lpns>>(msg);
        assert_eq!(lpp.as_str(), contract_addr);
        assert_eq!(1, funds.len());
        assert_eq!(Uint256::from(amount), funds[0].amount);
        assert!(matches!(exec, LppExecuteMsg::RepayLoan()));
    }

    fn assert_finalize(msg: &CosmosMsg, leaser: &Addr, customer: &Addr) {
        let (contract_addr, exec, funds) = as_execute::<FinalizerExecuteMsg>(msg);
        assert_eq!(leaser.as_str(), contract_addr);
        assert!(funds.is_empty());
        match exec {
            FinalizerExecuteMsg::FinalizeLease {
                customer: finalized,
            } => {
                assert_eq!(customer, &finalized)
            }
        }
    }

    fn assert_bank(msg: &CosmosMsg, to: &Addr, amount: Amount) {
        match msg {
            CosmosMsg::Bank(BankMsg::Send {
                to_address,
                amount: coins,
            }) => {
                assert_eq!(to.as_str(), to_address);
                assert_eq!(1, coins.len());
                assert_eq!(Uint256::from(amount), coins[0].amount);
            }
            other => panic!("expected a bank send, got {other:?}"),
        }
    }

    fn as_execute<M>(msg: &CosmosMsg) -> (&str, M, &[cosmwasm_std::Coin])
    where
        M: serde::de::DeserializeOwned,
    {
        match msg {
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr,
                msg,
                funds,
            }) => (
                contract_addr,
                cosmwasm_std::from_json(msg).expect("a decodable execute message"),
                funds,
            ),
            other => panic!("expected a wasm execute, got {other:?}"),
        }
    }

    #[test]
    fn guard_accepts_audited_state() {
        let mut storage = MockStorage::new();
        storage.set(STATE_KEY, AUDITED_STATE);

        assert!(guard(&storage).is_ok());
    }

    #[test]
    fn guard_rejects_absent_state() {
        let storage = MockStorage::new();

        assert!(matches!(
            guard(&storage),
            Err(ContractError::Issue654StateMismatch())
        ));
    }

    #[test]
    fn guard_rejects_mutated_state() {
        const FLIPPED_TAIL: usize = 4;

        let mut storage = MockStorage::new();
        let mut tampered = AUDITED_STATE.to_vec();
        let last_coin_index = tampered.len() - FLIPPED_TAIL;
        tampered[last_coin_index] = b'2';
        storage.set(STATE_KEY, &tampered);

        assert!(matches!(
            guard(&storage),
            Err(ContractError::Issue654StateMismatch())
        ));
    }
}
