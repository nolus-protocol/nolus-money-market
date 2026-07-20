use currency::test::{SubGroup, SubGroupTestC10, SuperGroup, SuperGroupTestC2};
use dex::Transport;
use finance::coin::{Coin, CoinDTO};
use platform::trx::Transaction;
use sdk::api::ProtobufAny;

use crate::{
    Impl,
    testing::{ExactAmountInSkel, SwapRequest},
};

#[test]
fn build_and_parse() {
    let expected_token_in = Coin::<SubGroupTestC10>::new(20).into();
    let expected_token_out = Coin::<SuperGroupTestC2>::new(2).into();

    let request: ProtobufAny = build_request(&expected_token_in, &expected_token_out);

    let SwapRequest {
        token_in,
        min_token_out: min_amount_out,
    } = <Impl as ExactAmountInSkel>::parse_request(request);

    assert_eq!(token_in, expected_token_in);
    assert_eq!(min_amount_out, expected_token_out.amount());
}

fn build_request(
    expected_token_in: &CoinDTO<SubGroup>,
    expected_token_out: &CoinDTO<SuperGroup>,
) -> ProtobufAny {
    let mut tx = Transaction::default();

    <Impl as Transport>::build_request(
        &mut tx,
        String::from("host_account").try_into().unwrap(),
        expected_token_in,
        expected_token_out,
    )
    .unwrap();

    let mut msgs = tx.into_iter();

    let msg = msgs.next().unwrap();

    assert!(msgs.next().is_none());

    msg
}
