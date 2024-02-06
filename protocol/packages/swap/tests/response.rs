use dex::swap::ExactAmountIn;
use swap::Impl;

#[test]
fn build_and_parse() {
    let expected_amount = 20;

    let mut resp = vec![<Impl as ExactAmountIn>::build_response(expected_amount)].into_iter();

    let parsed = <Impl as ExactAmountIn>::parse_response(&mut resp).unwrap();

    assert_eq!(parsed, expected_amount);

    assert_eq!(resp.next(), None);
}
