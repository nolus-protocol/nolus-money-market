use dex::swap::ExactAmountIn;
use finance::coin::Amount;

use crate::{Impl, testing::ExactAmountInSkel};

#[test]
fn build_and_parse() {
    const EXPECTED_AMOUNT: Amount = 20;

    let mut iter = Some(<Impl as ExactAmountInSkel>::build_response(EXPECTED_AMOUNT)).into_iter();

    assert_eq!(
        <Impl as ExactAmountIn>::parse_response(&mut iter).unwrap(),
        EXPECTED_AMOUNT
    );

    assert!(iter.next().is_none());
}
