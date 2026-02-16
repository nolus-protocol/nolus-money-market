use currency::test::{SubGroup, SubGroupTestC10, SuperGroup, SuperGroupTestC2, SuperGroupTestC3};
use dex::swap::{ExactAmountIn, SwapPathSlice};
use finance::coin::{Coin, CoinDTO};
use oracle::api::swap::SwapTarget;
use platform::trx::Transaction;
use sdk::{cosmos_sdk_proto::Any as CosmosAny, ica::ProtobufAny};

use crate::{
    Impl,
    testing::{ExactAmountInSkel, SwapRequest},
};

#[test]
fn build_and_parse() {
    let expected_token_in = Coin::<SubGroupTestC10>::new(20).into();
    let expected_token_out = Coin::<SuperGroupTestC2>::new(2).into();

    let expected_swap_path = vec![
        SwapTarget {
            pool_id: 0,
            target: currency::dto::<SuperGroupTestC2, _>(),
        },
        SwapTarget {
            pool_id: 0,
            target: currency::dto::<SuperGroupTestC3, _>(),
        },
    ];

    let request: CosmosAny =
        build_request(&expected_token_in, &expected_token_out, &expected_swap_path);

    let SwapRequest {
        token_in,
        min_token_out: min_amount_out,
        swap_path,
    } = <Impl as ExactAmountInSkel>::parse_request(request);

    assert_eq!(token_in, expected_token_in);
    assert_eq!(min_amount_out, expected_token_out.amount());
    assert_eq!(swap_path, expected_swap_path);
}

fn build_request(
    expected_token_in: &CoinDTO<SubGroup>,
    expected_token_out: &CoinDTO<SuperGroup>,
    expected_swap_path: SwapPathSlice<'_, SuperGroup>,
) -> CosmosAny {
    let mut tx = Transaction::default();

    <Impl as ExactAmountIn>::build_request(
        &mut tx,
        String::from("host_account").try_into().unwrap(),
        expected_token_in,
        expected_token_out,
        expected_swap_path,
    )
    .unwrap();

    let mut msgs = tx.into_iter();

    let msg: ProtobufAny = msgs.next().unwrap();

    assert!(msgs.next().is_none());

    CosmosAny::from(msg)
}
