use finance::coin::Amount;
use platform::trx;

use crate::{testing::ExactAmountInSkel, Impl};

pub fn build_two_responses(amount1: Amount, amount2: Amount) -> Vec<u8> {
    let responses = vec![Impl::build_response(amount1), Impl::build_response(amount2)];

    trx::encode_msg_responses(responses.into_iter())
}

#[cfg(test)]
mod test {
    use dex::swap::{Error, ExactAmountIn};
    use platform::trx;

    use crate::Impl;

    #[test]
    fn validate_two_responses() {
        let amount1 = 2000;
        let amount2 = 3124;
        let out = super::build_two_responses(amount1, amount2);

        let mut resp = trx::decode_msg_responses(&out).unwrap();

        assert_eq!(amount1, Impl::parse_response(&mut resp).unwrap());
        assert_eq!(amount2, Impl::parse_response(&mut resp).unwrap());
        assert!(matches!(
            Impl::parse_response(&mut resp).unwrap_err(),
            Error::MissingResponse(_)
        ));
        assert_eq!(None, resp.next());
    }
}
