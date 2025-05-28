use dex::MaxSlippage;
use lease::api::limits::MaxSlippages;
use leaser::{ContractError, msg::NewConfig};
use sdk::testing;

use crate::common::{
    LEASE_ADMIN, USER,
    leaser::{self as leaser_common, Instantiator},
};

#[test]
fn not_privileged() {
    let mut test_case = leaser_common::test_case();

    let user_addr = testing::user(USER);
    let leaser_addr = test_case.address_book.leaser().clone();

    assert!(matches!(
        test_case
            .app
            .execute(
                user_addr,
                leaser_addr,
                &leaser::msg::ExecuteMsg::ConfigLeases(Instantiator::new_config()),
                &[],
            )
            .expect_err("config by non authorized user should fail")
            .downcast_ref::<ContractError>(),
        Some(&ContractError::Unauthorized(_))
    ));
}

#[test]
fn privileged() {
    let mut test_case = leaser_common::test_case();

    let admin = testing::user(LEASE_ADMIN);
    let leaser_addr = test_case.address_book.leaser().clone();

    let new_config = NewConfig {
        lease_position_spec: Instantiator::position_spec(),
        lease_interest_rate_margin: Instantiator::INTEREST_RATE_MARGIN,
        lease_due_period: Instantiator::REPAYMENT_PERIOD,
        lease_max_slippages: MaxSlippages {
            liquidation: MaxSlippage::unchecked(Instantiator::MAX_SLIPPAGE),
        },
    };

    assert!(
        test_case
            .app
            .execute(
                admin,
                leaser_addr,
                &leaser::msg::ExecuteMsg::ConfigLeases(new_config),
                &[],
            )
            .is_ok()
    );
}
