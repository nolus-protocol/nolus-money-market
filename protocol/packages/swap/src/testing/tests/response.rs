use dex::swap::{Error, ExactAmountIn};
use finance::coin::Amount;
use platform::trx;

use crate::{testing::ExactAmountInSkel, Impl};

#[test]
fn build_and_parse() {
    let expected_amount = 20;

    let mut resp = vec![<Impl as ExactAmountInSkel>::build_response(expected_amount)].into_iter();

    let parsed = <Impl as ExactAmountIn>::parse_response(&mut resp).unwrap();

    assert_eq!(parsed, expected_amount);

    assert_eq!(resp.next(), None);
}

pub(crate) fn validate(resp_base64: &str, exp_amount1: Amount, exp_amount2: Amount) {
    use base64::{engine::general_purpose, Engine};

    let resp = general_purpose::STANDARD.decode(resp_base64).unwrap();
    let mut resp_messages = trx::decode_msg_responses(&resp).unwrap();

    assert_eq!(
        exp_amount1,
        Impl::parse_response(&mut resp_messages).unwrap()
    );
    assert_eq!(
        exp_amount2,
        Impl::parse_response(&mut resp_messages).unwrap()
    );
    assert!(matches!(
        Impl::parse_response(&mut resp_messages).unwrap_err(),
        Error::MissingResponse(_)
    ));
    assert_eq!(None, resp_messages.next());
}
