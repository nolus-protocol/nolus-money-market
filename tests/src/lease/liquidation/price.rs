use crate::common::testing;
use currencies::PaymentGroup;
use currency::CurrencyDef as _;
use finance::{
    coin::{Amount, CoinDTO},
    percent::{Percent, Percent100},
    zero::Zero as _,
};
use lease::api::query::StateResponse;
use sdk::{
    cosmwasm_std::{Addr, Event},
    cw_multi_test::AppResponse,
};

use crate::{
    common::{
        self, USER, lease as common_lease,
        leaser::{self, Instantiator as LeaserInstantiator},
        remote_lease_controller_stub as stub,
    },
    lease as lease_mod,
};

use super::super::{DOWNPAYMENT, LeaseCoin, LeaseCurrency, LpnCoin, LpnCurrency, PaymentCurrency};

// The warning tests open with a 45% max LTD so the literal-floor opening
// still lands in the no-warning zone (~36.5% LTV at the identity price);
// each test then drops the price into the targeted warning band.

#[test]
#[should_panic = "No liquidation warning emitted!"]
fn liquidation_warning_price_0() {
    liquidation_warning(
        // LTV ~71.6%, below the first warning level
        1000000,
        510000,
        LeaserInstantiator::liability().max(), //not used
        "N/A",
    );
}

#[test]
fn liquidation_warning_price_1() {
    liquidation_warning(
        // LTV ~74.5%
        1000000,
        490000,
        LeaserInstantiator::FIRST_LIQ_WARN,
        "1",
    );
}

#[test]
fn liquidation_warning_price_2() {
    liquidation_warning(
        // LTV ~76.1%
        1000000,
        480000,
        LeaserInstantiator::SECOND_LIQ_WARN,
        "2",
    );
}

#[test]
fn liquidation_warning_price_3() {
    liquidation_warning(
        // LTV ~79.0%
        1000000,
        462000,
        LeaserInstantiator::THIRD_LIQ_WARN,
        "3",
    );
}

#[test]
fn full_liquidation() {
    let mut test_case = lease_mod::create_test_case::<PaymentCurrency>();

    let lease_addr: Addr = lease_mod::open_lease(&mut test_case, DOWNPAYMENT, None);

    // the literal-floor opening: 85% of the downpayment quote plus 85% of
    // the borrow quote, truncated per swap leg
    let lease_amount: Amount = 2428571428570;
    let borrowed_amount: Amount = 1857142857142;

    // the base is chosen to be close to the asset amount to trigger a full
    // liquidation; the close swap now rides the controller, so the price
    // alarm emits no ICA `SwapExactIn` - `unwrap_response` would panic on a
    // non-empty ICA queue.
    let () = lease_mod::deliver_new_price(
        &mut test_case,
        common::coin(lease_amount - 2),
        common::coin(borrowed_amount),
    )
    .ignore_response()
    .unwrap_response();

    // The position asset sells for LPN on the remote account at the
    // max-slippage floor; the stand-in pays the price-derived (identity)
    // quote, i.e. the full asset amount in LPN.
    let swap = lease_mod::recorded_close_swap(&test_case, &lease_addr);
    assert_eq!(
        &CoinDTO::<PaymentGroup>::from(LeaseCoin::new(lease_amount)),
        swap.coin_in()
    );
    assert_eq!(
        currency::dto::<LpnCurrency, PaymentGroup>(),
        swap.min_out().currency()
    );

    let (proceeds, arrival): (LpnCoin, AppResponse) =
        lease_mod::settle_close_proceeds(&mut test_case, &lease_addr);
    assert_eq!(LpnCoin::new(lease_amount), proceeds);

    arrival.assert_event(&Event::new("wasm-ls-liquidation").add_attribute("loan-close", "true"));

    common_lease::assert_lease_balance_eq(
        &test_case.app,
        &lease_addr,
        common::cwcoin(LeaseCoin::ZERO),
    );

    let state = lease_mod::state_query(&test_case, lease_addr);
    assert!(
        matches!(state, StateResponse::Liquidated()),
        "should have been in Liquidated state"
    );
    leaser::assert_no_leases(
        &test_case.app,
        test_case.address_book.leaser().clone(),
        testing::user(USER),
    )
}

/// A full liquidation whose proceeds fall short of the outstanding loan
/// draws on the Reserve to settle it - the controller-path analogue of the
/// original ICA test that forced `liq_outcome = borrowed - 11123` via the
/// `do_swap` price callback. The stand-in pays the configured below-loan
/// output for the sell→LPN swap; the Reserve, pre-funded with the shortfall,
/// tops the repayment up to the full `borrowed_amount`.
#[test]
fn full_liquidation_reserve_covers_shortfall() {
    let mut test_case = lease_mod::create_test_case::<PaymentCurrency>();

    let lease_addr: Addr = lease_mod::open_lease(&mut test_case, DOWNPAYMENT, None);
    let controller = test_case.address_book.remote_lease_controller().clone();
    let reserve: Addr = test_case.address_book.reserve().clone();

    // the literal-floor opening: 85% of the downpayment quote plus 85% of
    // the borrow quote, truncated per swap leg
    let lease_amount: Amount = 2428571428570;
    let borrowed_amount: Amount = 1857142857142;
    let liq_outcome: Amount = borrowed_amount - 11123; // below the loan, to draw on the Reserve

    // Pre-fund the Reserve with exactly the shortfall it must cover.
    test_case.send_funds_from_admin(
        reserve.clone(),
        &[common::cwcoin_from_amount::<LpnCurrency>(
            borrowed_amount - liq_outcome,
        )],
    );

    // Force the liquidation's sell→LPN swap to pay below the loan; the
    // override is one-shot, so only this swap is affected.
    stub::set_next_swap_output(
        &mut test_case.app,
        &controller,
        CoinDTO::<PaymentGroup>::from(LpnCoin::new(liq_outcome)),
    );

    // the base is chosen to be close to the asset amount to trigger a full
    // liquidation; the close swap rides the controller, so the price alarm
    // emits no ICA `SwapExactIn` - `unwrap_response` would panic on a
    // non-empty ICA queue.
    let () = lease_mod::deliver_new_price(
        &mut test_case,
        common::coin(lease_amount - 2),
        common::coin(borrowed_amount),
    )
    .ignore_response()
    .unwrap_response();

    // The full position sells for LPN, but the stand-in paid the configured
    // below-loan output that drains home.
    let (proceeds, arrival): (LpnCoin, AppResponse) =
        lease_mod::settle_close_proceeds(&mut test_case, &lease_addr);
    assert_eq!(LpnCoin::new(liq_outcome), proceeds);

    // The Reserve top-up settles the full loan: the liquidation reports the
    // whole `borrowed_amount` repaid and the loan closed.
    arrival.assert_event(
        &Event::new("wasm-ls-liquidation")
            .add_attribute("payment-amount", borrowed_amount.to_string())
            .add_attribute("loan-close", "true"),
    );

    // The Reserve was drained of exactly the shortfall.
    assert!(
        platform::bank::balance::<LpnCurrency>(&reserve, test_case.app.query())
            .unwrap()
            .is_zero()
    );

    common_lease::assert_lease_balance_eq(
        &test_case.app,
        &lease_addr,
        common::cwcoin(LeaseCoin::ZERO),
    );

    let state = lease_mod::state_query(&test_case, lease_addr);
    assert!(
        matches!(state, StateResponse::Liquidated()),
        "should have been in Liquidated state"
    );
    leaser::assert_no_leases(
        &test_case.app,
        test_case.address_book.leaser().clone(),
        testing::user(USER),
    )
}

fn liquidation_warning(base: Amount, quote: Amount, liability: Percent100, level: &str) {
    let mut test_case = lease_mod::create_test_case::<PaymentCurrency>();
    let _lease =
        lease_mod::open_lease(&mut test_case, DOWNPAYMENT, Some(Percent::from_percent(45)));

    let response: AppResponse = lease_mod::deliver_new_price(
        &mut test_case,
        common::coin::<LeaseCurrency>(base),
        common::coin::<LpnCurrency>(quote),
    )
    .unwrap_response();

    let event = response
        .events
        .iter()
        .find(|event| event.ty == "wasm-ls-liquidation-warning")
        .expect("No liquidation warning emitted!");

    let attribute = event
        .attributes
        .iter()
        .find(|attribute| attribute.key == "customer")
        .expect("Customer attribute not present!");

    assert_eq!(attribute.value, testing::user(USER).to_string());

    let attribute = event
        .attributes
        .iter()
        .find(|attribute| attribute.key == "ltv")
        .expect("LTV attribute not present!");

    assert_eq!(attribute.value, liability.display_primitive());

    let attribute = event
        .attributes
        .iter()
        .find(|attribute| attribute.key == "level")
        .expect("Level attribute not present!");

    assert_eq!(attribute.value, level);

    let attribute = event
        .attributes
        .iter()
        .find(|attribute| attribute.key == "lease-asset")
        .expect("Lease Asset attribute not present!");

    assert_eq!(&attribute.value, LeaseCurrency::ticker());
}
