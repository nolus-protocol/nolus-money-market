use std::any::type_name;

use currency::Group;
use finance::coin::Amount;
use sdk::{api::ProtobufAny, cosmos_sdk_proto::prost::Message as _};

use crate::testing::{self, ExactAmountInSkel, SwapRequest};

use super::{Impl, RequestMsg, ResponseMsg, api::TypeUrl as _};

impl ExactAmountInSkel for Impl {
    fn parse_request<GIn>(request: ProtobufAny) -> SwapRequest<GIn>
    where
        GIn: Group,
    {
        let RequestMsg {
            sender: _,
            routes: _,
            token_in: Some(token_in),
            token_out_min_amount,
        } = parse_request_from_any_and_type_url(request)
        else {
            testing::pattern_match_else(type_name::<RequestMsg>())
        };

        let token_in = testing::parse_dex_token(&token_in.amount, &token_in.denom);

        SwapRequest {
            token_in,
            min_token_out: token_out_min_amount.parse().expect("valid amount integer"),
        }
    }

    fn build_response(amount_out: Amount) -> ProtobufAny {
        let resp = ResponseMsg {
            token_out_amount: amount_out.to_string(),
        };

        ProtobufAny::new(ResponseMsg::TYPE_URL, resp.encode_to_vec())
    }
}

fn parse_request_from_any_and_type_url(request: ProtobufAny) -> RequestMsg {
    assert!(
        request.of_type(RequestMsg::TYPE_URL),
        "Different type URL than expected one encountered!"
    );

    request.decode().expect("Expected a swap request message!")
}
