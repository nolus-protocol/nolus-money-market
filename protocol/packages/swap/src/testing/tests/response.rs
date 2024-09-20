use dex::swap::ExactAmountIn;
use finance::coin::Amount;

use crate::{testing::ExactAmountInSkel, Impl};

#[test]
fn build_and_parse() {
    const EXPECTED_AMOUNT: Amount = 20;

    assert_eq!(
        <Impl as ExactAmountIn>::parse_response(<Impl as ExactAmountInSkel>::build_response(
            EXPECTED_AMOUNT
        ),)
        .unwrap(),
        EXPECTED_AMOUNT
    );
}
