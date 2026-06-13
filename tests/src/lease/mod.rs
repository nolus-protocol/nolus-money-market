use currencies::{
    LeaseGroup, Lpn, PaymentGroup,
    testing::{LeaseC2, PaymentC1},
};
use currency::{Currency, CurrencyDef, MemberOf};
use dex::MaxSlippage;
use finance::{
    coin::{Amount, Coin},
    duration::Duration,
    interest,
    percent::{Percent, Percent100},
    price::{self, Price},
};
use lease::api::{
    ExecuteMsg,
    query::{ClosePolicy, StateResponse, opened::Status},
};
use leaser::msg::QuoteResponse;
use platform::coin_legacy;
use sdk::{
    cosmwasm_std::{Addr, coin},
    cw_multi_test::AppResponse,
    testing,
};

use crate::common::{
    self, ADDON_OPTIMAL_INTEREST_RATE, ADMIN, BASE_INTEREST_RATE, USER, UTILIZATION_OPTIMAL,
    leaser::{self as leaser_mod, Instantiator as LeaserInstantiator},
    protocols::Registry,
    test_case::{
        TestCase,
        app::App,
        builder::Builder as TestCaseBuilder,
        response::{RemoteChain, ResponseWithInterChainMsgs},
    },
};

mod close;
mod close_policy;
mod close_position;
mod compare_with_lpp;
mod heal;
mod liquidation;
mod open;
mod remote_lease_callback;
mod remote_lease_close;
// TODO #142 Phase 3: enable when the OpenLease state + Status::OpenFailed land.
mod remote_lease_open;
mod remote_lease_swap;
mod remote_lease_transfer_out;
mod repay;
mod slippage;

type LpnCurrency = Lpn;
type LpnCoin = Coin<LpnCurrency>;

type LeaseCurrency = LeaseC2;
type LeaseCoin = Coin<LeaseCurrency>;

type PaymentCurrency = PaymentC1;
type PaymentCoin = Coin<PaymentCurrency>;

const DOWNPAYMENT: PaymentCoin = PaymentCoin::new(1_000_000_000_000);

pub(super) type LeaseTestCase = TestCase<Addr, Addr, Addr, Addr, Addr, Addr, Addr, Addr>;

pub(super) fn create_payment_coin(amount: Amount) -> PaymentCoin {
    PaymentCoin::new(amount)
}

pub(super) fn price_lpn_of<C>() -> Price<C, LpnCurrency>
where
    C: Currency,
{
    Price::identity()
}

pub(super) fn feed_price<ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Lpp, TimeAlarms>(
    test_case: &mut TestCase<
        ProtocolsRegistry,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Lpp,
        Addr,
        TimeAlarms,
    >,
) {
    let lease_price = price_lpn_of::<LeaseCurrency>();
    common::oracle::feed_price_pair(test_case, testing::user(ADMIN), lease_price);

    let payment_price = price_lpn_of::<PaymentCurrency>();
    common::oracle::feed_price_pair(test_case, testing::user(ADMIN), payment_price);
}

pub(super) fn deliver_new_price(
    test_case: &mut LeaseTestCase,
    base: LeaseCoin,
    quote: LpnCoin,
) -> ResponseWithInterChainMsgs<'_, AppResponse> {
    let sender = testing::user(ADMIN);
    common::oracle::feed_price(test_case, sender.clone(), base, quote);

    common::oracle::dispatch(test_case, sender)
}

pub(super) fn create_test_case<InitFundsC>() -> LeaseTestCase
where
    InitFundsC: CurrencyDef,
{
    let mut test_case = TestCaseBuilder::<LpnCurrency, _, _, _, _, _, _, _, _>::with_reserve(&[
        common::cwcoin_from_amount::<PaymentCurrency>(10_000_000_000_000_000_000_000_000_000),
        common::cwcoin_dex::<PaymentCurrency>(10_000_000_000_000_000_000_000_000_000),
        common::cwcoin_from_amount::<LpnCurrency>(10_000_000_000_000_000_000_000_000_000),
        common::cwcoin_dex::<LpnCurrency>(10_000_000_000_000_000_000_000_000_000),
        common::cwcoin_from_amount::<LeaseCurrency>(10_000_000_000_000_000_000_000_000_000),
        common::cwcoin_dex::<LeaseCurrency>(10_000_000_000_000_000_000_000_000_000),
        common::cwcoin_from_amount::<InitFundsC>(10_000_000_000_000_000_000_000_000_000),
        common::cwcoin_dex::<InitFundsC>(10_000_000_000_000_000_000_000_000_000),
    ])
    .init_lpp_with_funds(
        None,
        &[coin(
            5_000_000_000_000_000_000_000_000_000,
            LpnCurrency::bank(),
        )],
        BASE_INTEREST_RATE,
        UTILIZATION_OPTIMAL,
        ADDON_OPTIMAL_INTEREST_RATE,
        TestCase::DEFAULT_LPP_MIN_UTILIZATION,
    )
    .init_time_alarms()
    .init_protocols_registry(Registry::SingleProtocol)
    .init_oracle(None)
    .init_treasury()
    .init_profit(24)
    .init_reserve()
    .init_leaser()
    .into_generic();

    test_case.send_funds_from_admin(
        testing::user(USER),
        &[common::cwcoin_from_amount::<InitFundsC>(
            1_000_000_000_000_000_000_000_000,
        )],
    );

    common::oracle::add_feeder(&mut test_case, testing::user(ADMIN));

    feed_price(&mut test_case);

    test_case
}

pub(super) fn calculate_interest(
    principal: Coin<LpnCurrency>,
    interest_rate: Percent100,
    duration: Duration,
) -> Coin<LpnCurrency> {
    interest::interest(interest_rate, principal, duration)
        .expect("Failed to calculate the interest")
}

pub(super) fn open_lease<
    ProtocolsRegistry,
    Treasury,
    Profit,
    Reserve,
    Lpp,
    Oracle,
    TimeAlarms,
    DownpaymentC,
>(
    test_case: &mut TestCase<
        ProtocolsRegistry,
        Treasury,
        Profit,
        Reserve,
        Addr,
        Lpp,
        Oracle,
        TimeAlarms,
    >,
    downpayment: Coin<DownpaymentC>,
    max_ltd: Option<Percent>,
) -> Addr
where
    DownpaymentC: CurrencyDef,
    DownpaymentC::Group: MemberOf<PaymentGroup>,
{
    let lease = try_init_lease(test_case, downpayment, max_ltd);
    complete_init_lease(test_case, downpayment, max_ltd, &lease);
    lease
}

pub(super) fn try_init_lease<
    ProtocolsRegistry,
    Treasury,
    Profit,
    Reserve,
    Lpp,
    Oracle,
    TimeAlarms,
    D,
>(
    test_case: &mut TestCase<
        ProtocolsRegistry,
        Treasury,
        Profit,
        Reserve,
        Addr,
        Lpp,
        Oracle,
        TimeAlarms,
    >,
    downpayment: Coin<D>,
    max_ltd: Option<Percent>,
) -> Addr
where
    D: CurrencyDef,
{
    let downpayment = (!downpayment.is_zero()).then(|| common::cwcoin::<D>(downpayment));

    let mut response = test_case
        .app
        .execute(
            testing::user(USER),
            test_case.address_book.leaser().clone(),
            &leaser::msg::ExecuteMsg::OpenLease {
                currency: currency::dto::<LeaseCurrency, _>(),
                max_ltd,
            },
            downpayment.as_slice(),
        )
        .unwrap();

    response.expect_register_ica(TestCase::DEX_CONNECTION_ID, TestCase::LEASE_ICA_ID);
    () = response.ignore_response().unwrap_response();

    leaser_mod::expect_a_lease(
        &test_case.app,
        test_case.address_book.leaser().clone(),
        testing::user(USER),
    )
}

pub(super) fn complete_init_lease<
    ProtocolsRegistry,
    Treasury,
    Profit,
    Reserve,
    Lpp,
    Oracle,
    TimeAlarms,
    DownpaymentC,
>(
    test_case: &mut TestCase<
        ProtocolsRegistry,
        Treasury,
        Profit,
        Reserve,
        Addr,
        Lpp,
        Oracle,
        TimeAlarms,
    >,
    downpayment: Coin<DownpaymentC>,
    max_ltd: Option<Percent>,
    lease: &Addr,
) where
    DownpaymentC: CurrencyDef,
    DownpaymentC::Group: MemberOf<PaymentGroup>,
{
    let quote: QuoteResponse = common::leaser::query_quote::<DownpaymentC, LeaseCurrency>(
        &test_case.app,
        test_case.address_book.leaser().clone(),
        downpayment,
        max_ltd,
    );
    let exp_borrow: LpnCoin = quote.borrow.try_into().unwrap();

    let controller = test_case.address_book.remote_lease_controller().clone();
    common::lease::complete_initialization(
        &mut test_case.app,
        TestCase::DEX_CONNECTION_ID,
        &controller,
        lease.clone(),
        downpayment,
        exp_borrow,
    );
}

pub(super) fn quote_borrow<
    ProtocolsRegistry,
    Treasury,
    Profit,
    Reserve,
    Lpp,
    Oracle,
    TimeAlarms,
>(
    test_case: &TestCase<
        ProtocolsRegistry,
        Treasury,
        Profit,
        Reserve,
        Addr,
        Lpp,
        Oracle,
        TimeAlarms,
    >,
    downpayment: PaymentCoin,
) -> LpnCoin {
    LpnCoin::try_from(quote_query(test_case, downpayment).borrow).unwrap()
}

pub(super) fn quote_query<
    ProtocolsRegistry,
    Treasury,
    Profit,
    Reserve,
    Lpp,
    Oracle,
    TimeAlarms,
    DownpaymentC,
>(
    test_case: &TestCase<
        ProtocolsRegistry,
        Treasury,
        Profit,
        Reserve,
        Addr,
        Lpp,
        Oracle,
        TimeAlarms,
    >,
    downpayment: Coin<DownpaymentC>,
) -> QuoteResponse
where
    DownpaymentC: CurrencyDef,
    DownpaymentC::Group: MemberOf<PaymentGroup>,
{
    common::leaser::query_quote::<_, LeaseCurrency>(
        &test_case.app,
        test_case.address_book.leaser().clone(),
        downpayment,
        None,
    )
}

pub(super) fn state_query<
    ProtocolsRegistry,
    Treasury,
    Profit,
    Reserve,
    Leaser,
    Lpp,
    Oracle,
    TimeAlarms,
>(
    test_case: &TestCase<
        ProtocolsRegistry,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Lpp,
        Oracle,
        TimeAlarms,
    >,
    lease: Addr,
) -> StateResponse {
    common::lease::fetch_state(&test_case.app, lease)
}

pub(super) fn expected_open_state<
    ProtocolsRegistry,
    Treasury,
    Profit,
    Reserve,
    Lpp,
    Oracle,
    TimeAlarms,
    DownpaymentC,
    PaymentC,
    AssetC,
>(
    test_case: &TestCase<
        ProtocolsRegistry,
        Treasury,
        Profit,
        Reserve,
        Addr,
        Lpp,
        Oracle,
        TimeAlarms,
    >,
    downpayment: Coin<DownpaymentC>,
    payments: Coin<PaymentC>,
    closed: Coin<AssetC>,
    max_due: Duration,
) -> StateResponse
where
    DownpaymentC: CurrencyDef,
    DownpaymentC::Group: MemberOf<PaymentGroup>,
    PaymentC: Currency + MemberOf<PaymentGroup>,
    AssetC: CurrencyDef,
    AssetC::Group: MemberOf<LeaseGroup>,
{
    let now = crate::block_time(test_case);
    let last_paid = now;
    let quote_result = quote_query(test_case, downpayment);
    let borrow: LpnCoin = quote_result.borrow.try_into().unwrap();
    let total: Coin<AssetC> = expected_opened_amount(downpayment, borrow);
    let expected_principal: LpnCoin =
        borrow - price::total(payments, price_lpn_of::<PaymentC>()).unwrap();
    let due_period_start = (now - max_due).max(last_paid);
    let (overdue, due) = (
        Duration::between(&last_paid, &due_period_start),
        Duration::between(&due_period_start, &now),
    );
    StateResponse::Opened {
        amount: (total - closed).into(),
        loan_interest_rate: quote_result.annual_interest_rate,
        margin_interest_rate: quote_result.annual_interest_rate_margin,
        principal_due: expected_principal.into(),
        overdue_margin: calculate_interest(
            expected_principal,
            quote_result.annual_interest_rate_margin,
            overdue,
        )
        .into(),
        overdue_interest: calculate_interest(
            expected_principal,
            quote_result.annual_interest_rate,
            overdue,
        )
        .into(),
        overdue_collect_in: if overdue == Duration::default() {
            Duration::between(&(now - max_due), &last_paid)
        } else {
            Duration::default()
        },
        due_margin: calculate_interest(
            expected_principal,
            quote_result.annual_interest_rate_margin,
            due,
        )
        .into(),
        due_interest: calculate_interest(
            expected_principal,
            quote_result.annual_interest_rate,
            due,
        )
        .into(),
        due_projection: Duration::default(),
        close_policy: ClosePolicy::default(),
        validity: now,
        status: Status::Idle,
    }
}

/// The asset amount a newly-opened lease ends up with under the
/// literal-floor swap model
///
/// A downpayment already denominated in the lease currency folds in
/// verbatim without a swap; every other coin swaps to exactly the
/// slippage-bounded floor of its oracle quote — the controller stand-in
/// pays `min_out` to the letter.
pub(super) fn expected_opened_amount<DownpaymentC, AssetC>(
    downpayment: Coin<DownpaymentC>,
    borrow: LpnCoin,
) -> Coin<AssetC>
where
    DownpaymentC: CurrencyDef,
    DownpaymentC::Group: MemberOf<PaymentGroup>,
    AssetC: CurrencyDef,
    AssetC::Group: MemberOf<LeaseGroup>,
{
    let borrow_floor = swap_min_out(price::total(borrow, price_lpn_of::<AssetC>().inv()).unwrap());
    if currency::equal::<DownpaymentC, AssetC>() {
        downpayment.coerce_into() + borrow_floor
    } else {
        let downpayment_quote = price::total(
            price::total(downpayment, price_lpn_of::<DownpaymentC>()).unwrap(),
            price_lpn_of::<AssetC>().inv(),
        )
        .unwrap();
        swap_min_out(downpayment_quote) + borrow_floor
    }
}

pub(super) fn swap_min_out<AssetC>(quote: Coin<AssetC>) -> Coin<AssetC>
where
    AssetC: Currency,
{
    MaxSlippage::unchecked(LeaserInstantiator::MAX_SLIPPAGE).min_out(quote)
}

pub(super) fn expected_newly_opened_state<
    ProtocolsRegistry,
    Treasury,
    Profit,
    Reserve,
    Lpp,
    Oracle,
    TimeAlarms,
    DownpaymentC,
    PaymentC,
>(
    test_case: &TestCase<
        ProtocolsRegistry,
        Treasury,
        Profit,
        Reserve,
        Addr,
        Lpp,
        Oracle,
        TimeAlarms,
    >,
    downpayment: Coin<DownpaymentC>,
    payments: Coin<PaymentC>,
) -> StateResponse
where
    DownpaymentC: CurrencyDef,
    DownpaymentC::Group: MemberOf<PaymentGroup>,
    PaymentC: Currency + MemberOf<PaymentGroup>,
{
    expected_open_state(
        test_case,
        downpayment,
        payments,
        Coin::<LeaseCurrency>::default(),
        LeaserInstantiator::REPAYMENT_PERIOD,
    )
}

/// Open a lease and repay the whole loan, driving the lease to `Paid` and
/// emitting the close-leg transfer-out. The repay-proceeds drain acks
/// inline (default `Ok`), so by the time this returns only the close
/// transfer-out is in flight.
pub(super) fn open_and_repay_fully(
    test_case: &mut LeaseTestCase,
) -> (Addr, LeaseCoin, AppResponse) {
    open_and_repay_fully_then(test_case, |_app| {})
}

/// As [`open_and_repay_fully`], but runs `pre_close_hook` after the
/// repay-proceeds drain has acked and before the funds-arrival alarm
/// triggers the close-leg transfer-out. A close-leg driver sets its
/// `op_tag::TRANSFER_OUT` `ResponseMode` here so the mode applies only to
/// the close transfer-out, not to the repay drain that precedes it (both
/// share the op tag).
pub(super) fn open_and_repay_fully_then<PreCloseHook>(
    test_case: &mut LeaseTestCase,
    pre_close_hook: PreCloseHook,
) -> (Addr, LeaseCoin, AppResponse)
where
    PreCloseHook: FnOnce(&mut App),
{
    let downpayment = DOWNPAYMENT;
    let lease = open_lease(test_case, downpayment, None);

    let borrowed_lpn = quote_borrow(test_case, downpayment);
    let borrowed: PaymentCoin =
        price::total(borrowed_lpn, price_lpn_of::<PaymentCurrency>().inv()).unwrap();
    let expected_funds: LeaseCoin = expected_opened_amount(downpayment, borrowed_lpn);

    let repay_response =
        repay::repay_with_hook_on_swap(test_case, lease.clone(), borrowed, pre_close_hook)
            .unwrap_response();
    (lease, expected_funds, repay_response)
}

/// Mirror the acknowledged transfer onto the bank balances: the remote
/// account (stood in by the ICA address) escrows the asset and the paired
/// ICS-20 channel lands it on the lease's local account
pub(super) fn settle_arrival(test_case: &mut LeaseTestCase, lease: &Addr, funds: LeaseCoin) {
    let ica_addr: Addr = TestCase::ica_addr(lease, TestCase::LEASE_ICA_ID);
    test_case
        .app
        .send_tokens(
            ica_addr,
            testing::user(ADMIN),
            &[coin_legacy::to_cosmwasm_on_dex(funds)],
        )
        .unwrap();
    test_case
        .app
        .send_tokens(
            testing::user(ADMIN),
            lease.clone(),
            &[common::cwcoin(funds)],
        )
        .unwrap();
}

pub(super) fn heal(test_case: &mut LeaseTestCase, lease: Addr) -> AppResponse {
    test_case
        .app
        .execute(testing::user(USER), lease, &ExecuteMsg::Heal(), &[])
        .unwrap()
        .unwrap_response()
}
