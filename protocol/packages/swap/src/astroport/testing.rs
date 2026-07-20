use std::any;

use currency::Group;
use dex::Transport;
use finance::coin::{Amount, CoinDTO};
use sdk::{
    api::ProtobufAny,
    cosmos_sdk_proto::{
        cosmos::base::v1beta1::Coin as ProtoCoin,
        prost::{Message as _, Name as _},
    },
    cosmwasm_std,
};

use crate::testing::{self, ExactAmountInSkel, SwapRequest};

use super::{
    GenericImpl, RequestMsg, ResponseMsg,
    api::{ExecuteMsg, SwapResponseData},
    router::Router,
};

impl<R> ExactAmountInSkel for GenericImpl<R>
where
    Self: Transport,
    R: Router,
{
    fn parse_request<GIn>(request: ProtobufAny) -> SwapRequest<GIn>
    where
        GIn: Group,
    {
        let RequestMsg {
            sender: _,
            contract,
            msg,
            funds,
        } = parse_request_from_any(request);

        assert_eq!(
            contract,
            R::ADDRESS,
            "Expected message to be addressed to currently selected router!"
        );

        let token_in = parse_one_token_from_vec::<GIn>(funds);

        let ExecuteMsg::ExecuteSwapOperations {
            operations: _operations,
            minimum_receive: Some(min_token_out),
            to: None,
            max_spread: Some(super::MAX_IMPACT),
        } = cosmwasm_std::from_json(msg).unwrap_or_else(|_| {
            panic!(
                r#"Expected message to be from type "{}""#,
                any::type_name::<ExecuteMsg>()
            )
        })
        else {
            testing::pattern_match_else(any::type_name::<RequestMsg>())
        };

        SwapRequest {
            token_in,
            min_token_out: min_token_out.into(),
        }
    }

    fn build_response(amount_out: Amount) -> ProtobufAny {
        let swap_resp = cosmwasm_std::to_json_vec(&SwapResponseData {
            return_amount: amount_out.into(),
        })
        .expect("test result serialization works");

        ProtobufAny::new(
            ResponseMsg::type_url(),
            (ResponseMsg { data: swap_resp }).encode_to_vec(),
        )
    }
}

fn parse_request_from_any(request: ProtobufAny) -> RequestMsg {
    request.decode().expect("Expected a swap request message!")
}

fn parse_one_token_from_vec<G>(funds: Vec<ProtoCoin>) -> CoinDTO<G>
where
    G: Group,
{
    if let [token_in] = funds.as_slice() {
        testing::parse_dex_token(&token_in.amount, &token_in.denom)
    } else {
        unimplemented!("Expected only one type of token!");
    }
}
