use ::lease::api::{ExecuteMsg, FullClose, PositionClose, StateResponse};
use currency::Currency;
use finance::{
    coin::{Amount, Coin},
    price,
};
use sdk::cosmwasm_std::{Addr, Event};

use crate::{
    common::{
        leaser,
        test_case::{response::ResponseWithInterChainMsgs, TestCase},
        USER,
    },
    lease::{
        self, dex, LeaseCoin, LeaseCurrency, Lpn, LpnCoin, PaymentCoin, PaymentCurrency,
        DOWNPAYMENT,
    },
};

#[test]
fn full_close() {
    let customer = Addr::unchecked(USER);
    let mut test_case = lease::create_test_case::<PaymentCurrency>();
    let user_balance_before: PaymentCoin = user_balance(&customer, &test_case);
    let lease = lease::open_lease(&mut test_case, DOWNPAYMENT, None);

    let borrow: LpnCoin = 1857142857142.into();
    let lease_amount: LeaseCoin = 2857142857142.into();
    let closed_amount: LeaseCoin = lease_amount;
    let closed_amount_in_lpn = price::total(closed_amount, lease::price_lpn_of());
    let response_close = send_full_close(&mut test_case, lease.clone());

    dex::expect_swap(response_close);
    let response_swap = dex::do_swap(
        &mut test_case,
        lease.clone(),
        closed_amount,
        closed_amount_in_lpn,
    );

    dex::expect_init_transfer_in(response_swap);
    let response_transfer_in = dex::do_transfer_in(
        &mut test_case,
        lease.clone(),
        closed_amount_in_lpn,
        Option::<LeaseCoin>::None,
    );

    response_transfer_in.assert_event(
        &Event::new("wasm-ls-close-position")
            .add_attribute("to", lease.clone())
            .add_attribute("payment-amount", Amount::from(closed_amount).to_string())
            .add_attribute("payment-symbol", Lpn::TICKER)
            .add_attribute("loan-close", true.to_string())
            .add_attribute("principal", Amount::from(borrow).to_string())
            .add_attribute("change", Amount::from(DOWNPAYMENT).to_string())
            .add_attribute("amount-amount", Amount::from(lease_amount).to_string())
            .add_attribute("amount-symbol", LeaseCurrency::TICKER),
    );

    assert_eq!(
        test_case
            .app
            .query()
            .query_all_balances(lease.clone())
            .unwrap(),
        &[],
    );

    let state = lease::state_query(&test_case, lease.as_str());
    assert_eq!(StateResponse::Closed(), state);

    leaser::assert_no_leases(
        &test_case.app,
        test_case.address_book.leaser().clone(),
        customer.clone(),
    );

    assert_eq!(
        user_balance_before - DOWNPAYMENT,
        user_balance::<PaymentCurrency>(&customer, &test_case)
    );
    assert_eq!(
        LpnCoin::from(Amount::from(DOWNPAYMENT)),
        user_balance::<Lpn>(&customer, &test_case)
    )
}

fn send_full_close<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle>(
    test_case: &mut TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, Addr>,
    contract_addr: Addr,
) -> ResponseWithInterChainMsgs<'_, ()> {
    test_case
        .app
        .execute(
            Addr::unchecked(USER),
            contract_addr.clone(),
            &ExecuteMsg::ClosePosition(PositionClose::FullClose(FullClose {})),
            &[],
        )
        .unwrap()
        .ignore_response()
}

fn user_balance<C>(
    customer: &Addr,
    test_case: &TestCase<(), Addr, Addr, Addr, Addr, Addr, Addr>,
) -> Coin<C>
where
    C: Currency,
{
    platform::bank::balance::<C>(customer, &test_case.app.query()).unwrap()
}
