use currency::test::{SubGroup, SubGroupTestC10, SuperGroup, SuperGroupTestC2, SuperGroupTestC3};
use dex::swap::ExactAmountIn;
use finance::coin::{Coin, CoinDTO};
use oracle::api::swap::SwapTarget;
use platform::trx::Transaction;
use sdk::{
    cosmos_sdk_proto::Any as CosmosAny, neutron_sdk::bindings::types::ProtobufAny as NeutronAny,
};

use crate::{
    Impl,
    testing::{ExactAmountInSkel, SwapRequest},
};

#[test]
fn build_and_parse() {
    let expected_token_in: CoinDTO<SubGroup> = Coin::<SubGroupTestC10>::new(20).into();

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

    let request: CosmosAny = build_request(expected_token_in, expected_swap_path.clone());

    let SwapRequest {
        token_in,
        swap_path,
    } = <Impl as ExactAmountInSkel>::parse_request::<SubGroup, SuperGroup>(request);

    assert_eq!(token_in, expected_token_in);
    assert_eq!(swap_path, expected_swap_path);
}

fn build_request(
    expected_token_in: CoinDTO<SubGroup>,
    expected_swap_path: Vec<SwapTarget<SuperGroup>>,
) -> CosmosAny {
    let mut tx = Transaction::default();

    <Impl as ExactAmountIn>::build_request::<SubGroup, SuperGroup>(
        &mut tx,
        String::from("host_account").try_into().unwrap(),
        &expected_token_in,
        &expected_swap_path,
    )
    .unwrap();

    let mut msgs = tx.into_iter();

    let NeutronAny { type_url, value } = msgs.next().unwrap();

    assert!(msgs.next().is_none());

    CosmosAny {
        type_url,
        value: value.into(),
    }
}
