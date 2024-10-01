use dex::swap::ExactAmountIn;
use finance::coin::Amount;

use crate::{testing::ExactAmountInSkel, Impl};

#[test]
fn build_and_parse() {
    let expected_amount = 20;

    let mut resp = vec![<Impl as ExactAmountInSkel>::build_response(expected_amount)].into_iter();

    let parsed = <Impl as ExactAmountIn>::parse_response(&mut resp).unwrap();

    assert_eq!(parsed, expected_amount);

    assert_eq!(resp.next(), None);
}
