use cosmwasm_std::{coins, Addr, Coin};
use cw_multi_test::Executor;
use finance::coin::{Usdc, Currency};

use crate::common::{test_case::TestCase, USER};

#[test]
#[should_panic(expected = "Unauthorized contract Id")]
fn open_loan_unauthorized_contract_id() {
    let denom = Usdc::SYMBOL;
    let user_addr = Addr::unchecked(USER);

    let mut test_case = TestCase::new(denom);
    test_case.init(&user_addr, coins(500, denom));
    test_case.init_lpp(None);

    //redeploy lease contract to change the code_id
    test_case.init_lease();

    let lease_addr = test_case.get_lease_instance();

    test_case
        .app
        .execute_contract(
            lease_addr,
            test_case.lpp_addr.unwrap(),
            &lpp::msg::ExecuteMsg::OpenLoan {
                amount: Coin::new(100, denom),
            },
            &coins(200, denom),
        )
        .unwrap();
}

#[test]
#[should_panic(expected = "No liquidity")]
fn open_loan_no_liquidity() {
    let denom = Usdc::SYMBOL;
    let user_addr = Addr::unchecked(USER);

    let mut test_case = TestCase::new(denom);
    test_case.init(&user_addr, coins(500, denom));
    test_case.init_lpp(None);

    let lease_addr = test_case.get_lease_instance();

    test_case
        .app
        .execute_contract(
            lease_addr,
            test_case.lpp_addr.unwrap(),
            &lpp::msg::ExecuteMsg::OpenLoan {
                amount: Coin::new(100, denom),
            },
            &coins(200, denom),
        )
        .unwrap();
}
