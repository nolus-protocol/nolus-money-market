use access_control::error::Error as AccessControlError;
use currencies::Lpn;
use finance::{
    coin::{Amount, Coin},
    zero::Zero,
};
use platform::{bank, contract::Code, error::Error as PlatformError};
use reserve::{
    api::{ConfigResponse, LpnCurrencyDTO, QueryMsg},
    error::Error as ReserveError,
};
use sdk::{cosmwasm_std::Addr, cw_multi_test::AppResponse, testing};

use crate::{
    common::{
        cwcoin, cwcoin_from_amount,
        leaser::Instantiator as LeaserInstantiator,
        test_case::{
            TestCase, app::App, builder::BlankBuilder as TestCaseBuilder,
            response::ResponseWithInterChainMsgs,
        },
    },
    lease::LeaseTestCase,
};

use super::lease;

type ReserveTest = TestCase<(), (), (), Addr, (), (), (), ()>;

#[test]
fn instantiate() {
    let test_case = TestCaseBuilder::<Lpn>::new().init_reserve().into_generic();
    let reserve = test_case.address_book.reserve().clone();
    assert_lpn(&test_case, reserve.clone(), &currency::dto::<Lpn, _>());
    assert_config(
        &test_case,
        reserve,
        &ConfigResponse::new(test_case.address_book.lease_code()),
    );
}

#[test]
fn new_lease_code() {
    let mut test_case = TestCaseBuilder::<Lpn>::new().init_reserve().into_generic();
    let reserve = test_case.address_book.reserve().clone();
    let new_lease_code = Code::unchecked(12);
    let err = set_new_lease_code(
        &mut test_case.app,
        reserve.clone(),
        testing::user("NOT_THE_LEASE_CODE_ADMIN"),
        new_lease_code,
    )
    .unwrap_err();
    assert!(matches!(
        err.downcast_ref::<ReserveError>(),
        Some(&ReserveError::Unauthorized(_))
    ));

    let resp = set_new_lease_code(
        &mut test_case.app,
        reserve.clone(),
        LeaserInstantiator::expected_addr(),
        new_lease_code,
    )
    .unwrap()
    .unwrap_response();
    assert_eq!(AppResponse::default().data, resp.data);

    assert_lpn(&test_case, reserve.clone(), &currency::dto::<Lpn, _>());
    assert_config(&test_case, reserve, &ConfigResponse::new(new_lease_code));
}

#[test]
fn cover_losses_unauthortized() {
    let mut test_case = lease::create_test_case::<Lpn>();
    let reserve = test_case.address_book.reserve().clone();
    let losses = 412314;
    let err = cover_losses_err(
        &mut test_case,
        reserve.clone(),
        testing::user("NOT_A_LEASE"),
        losses,
    );
    assert!(matches!(
        err.downcast_ref::<ReserveError>(),
        Some(&ReserveError::Platform(
            PlatformError::CosmWasmQueryContractInfo(_)
        ))
    ));

    let unauthorized_sender = test_case.address_book.reserve().clone();
    let err = cover_losses_err(&mut test_case, reserve, unauthorized_sender, losses);
    assert!(matches!(
        err.downcast_ref::<ReserveError>(),
        Some(&ReserveError::Platform(PlatformError::UnexpectedCode(_, _)))
    ));
}

#[test]
fn cover_losses_insufficient_balance() {
    let mut test_case: LeaseTestCase = lease::create_test_case::<Lpn>();
    let downpayment = Coin::<Lpn>::new(1_000_000);
    let lease_addr: Addr = lease::open_lease(&mut test_case, downpayment, None);

    let reserve = test_case.address_book.reserve().clone();

    let losses = 1;
    let err = cover_losses_err(&mut test_case, reserve, lease_addr, losses);
    assert!(matches!(
        err.downcast_ref(),
        Some(ReserveError::InsufficientBalance)
    ));
}

#[test]
fn cover_losses_enough_balance() {
    let mut test_case: LeaseTestCase = lease::create_test_case::<Lpn>();
    let downpayment = Coin::<Lpn>::new(1_000_000);
    let lease_addr: Addr = lease::open_lease(&mut test_case, downpayment, None);

    let reserve = test_case.address_book.reserve().clone();
    let losses = 1425;
    test_case.send_funds_from_admin(reserve.clone(), &[cwcoin_from_amount::<Lpn>(losses)]);

    let _resp = cover_losses_ok(&mut test_case, reserve.clone(), lease_addr, losses);
    assert_balance_eq(&test_case, &reserve, Coin::ZERO);
}

#[test]
fn dump_balance_unauthorized() {
    let mut test_case = lease::create_test_case::<Lpn>();
    let reserve = test_case.address_book.reserve().clone();
    let receiver = testing::user("USER");

    let msg = reserve::api::ExecuteMsg::DumpBalanceTo(receiver);
    let err = test_case
        .app
        .execute(testing::user("malicious user"), reserve, &msg, &[])
        .unwrap_err();

    assert!(matches!(
        err.downcast_ref::<ReserveError>(),
        Some(&ReserveError::Unauthorized(
            AccessControlError::Unauthorized {}
        ))
    ));
}

#[test]
fn dump_balance_ok() {
    let mut test_case = lease::create_test_case::<Lpn>();
    let reserve = test_case.address_book.reserve().clone();
    let receiver = testing::user("USER");
    let balance = Coin::new(4213141);

    test_case.send_funds_from_admin(reserve.clone(), &[cwcoin::<Lpn>(balance)]);

    let msg = reserve::api::ExecuteMsg::DumpBalanceTo(receiver.clone());
    let _resp = test_case
        .app
        .execute(
            LeaserInstantiator::expected_addr(),
            reserve.clone(),
            &msg,
            &[],
        )
        .unwrap()
        .unwrap_response();

    assert_balance_eq(&test_case, &reserve, Coin::ZERO);
    assert_balance_eq(&test_case, &receiver, balance);
}

#[test]
fn dump_zero_balance() {
    let mut test_case = lease::create_test_case::<Lpn>();
    let reserve = test_case.address_book.reserve().clone();
    let receiver = testing::user("USER");

    assert_balance_eq(&test_case, &reserve, Coin::ZERO);

    let msg = reserve::api::ExecuteMsg::DumpBalanceTo(receiver.clone());
    let _resp = test_case
        .app
        .execute(
            LeaserInstantiator::expected_addr(),
            reserve.clone(),
            &msg,
            &[],
        )
        .unwrap()
        .unwrap_response();

    assert_balance_eq(&test_case, &reserve, Coin::ZERO);
}

fn cover_losses_err(
    test_case: &mut LeaseTestCase,
    reserve: Addr,
    sender: Addr,
    losses: Amount,
) -> anyhow::Error {
    do_cover_losses(losses, test_case, sender, reserve).unwrap_err()
}

fn cover_losses_ok(
    test_case: &mut LeaseTestCase,
    reserve: Addr,
    sender: Addr,
    losses: Amount,
) -> AppResponse {
    do_cover_losses(losses, test_case, sender, reserve)
        .unwrap()
        .unwrap_response()
}

fn do_cover_losses(
    losses: Amount,
    test_case: &mut LeaseTestCase,
    sender: Addr,
    reserve: Addr,
) -> anyhow::Result<ResponseWithInterChainMsgs<'_, AppResponse>> {
    let msg = reserve::api::ExecuteMsg::CoverLiquidationLosses(Coin::<Lpn>::new(losses).into());

    test_case.app.execute(sender, reserve, &msg, &[])
}

fn set_new_lease_code(
    app: &mut App,
    reserve: Addr,
    sender: Addr,
    new_lease_code: Code,
) -> anyhow::Result<ResponseWithInterChainMsgs<'_, AppResponse>> {
    let msg = reserve::api::ExecuteMsg::NewLeaseCode(new_lease_code);
    app.execute(sender, reserve.clone(), &msg, &[])
}

fn assert_config(test: &ReserveTest, reserve: Addr, exp_config: &ConfigResponse) {
    let cfg: ConfigResponse = test
        .app
        .query()
        .query_wasm_smart(reserve, &QueryMsg::Config())
        .unwrap();
    assert_eq!(exp_config, &cfg);
}

fn assert_lpn(test: &ReserveTest, reserve: Addr, exp_lpn: &LpnCurrencyDTO) {
    let cfg: LpnCurrencyDTO = test
        .app
        .query()
        .query_wasm_smart(reserve, &QueryMsg::ReserveLpn())
        .unwrap();
    assert_eq!(exp_lpn, &cfg);
}

fn assert_balance_eq(test: &LeaseTestCase, account: &Addr, expected_balance: Coin<Lpn>) {
    assert_eq!(
        Ok(expected_balance),
        bank::balance::<Lpn>(account, test.app.query())
    );
}
