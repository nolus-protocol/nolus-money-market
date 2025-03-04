use std::any::type_name;

use osmosis_std::types::osmosis::poolmanager::v1beta1::SwapAmountInRoute;

use currency::Group;
use finance::coin::Amount;
use oracle::api::swap::SwapTarget;
use sdk::{cosmos_sdk_proto::Any as CosmosAny, cosmos_sdk_proto::prost::Message as _};

use crate::testing::{self, ExactAmountInSkel, SwapRequest};

use super::{Impl, RequestMsg, ResponseMsg};

impl ExactAmountInSkel for Impl {
    fn parse_request<GIn, GSwap>(request: CosmosAny) -> SwapRequest<GIn, GSwap>
    where
        GIn: Group,
        GSwap: Group,
    {
        let RequestMsg {
            sender: _,
            routes,
            token_in: Some(token_in),
            token_out_min_amount,
        } = parse_request_from_any_and_type_url(request)
        else {
            testing::pattern_match_else(type_name::<RequestMsg>())
        };

        assert_eq!({ token_out_min_amount }, "1");

        let token_in = testing::parse_dex_token(&token_in.amount, &token_in.denom);

        SwapRequest {
            token_in,
            swap_path: routes
                .into_iter()
                .map(
                    |SwapAmountInRoute {
                         pool_id,
                         token_out_denom: target,
                     }| {
                        SwapTarget {
                            pool_id,
                            target: testing::from_dex_symbol(&target)
                                .expect("Asked asset doesn't belong to swapping currency group!"),
                        }
                    },
                )
                .collect(),
        }
    }

    fn build_response(amount_out: Amount) -> CosmosAny {
        let resp = ResponseMsg {
            token_out_amount: amount_out.to_string(),
        };

        CosmosAny {
            type_url: ResponseMsg::TYPE_URL.into(),
            value: resp.encode_to_vec(),
        }
    }
}

fn parse_request_from_any_and_type_url(request: CosmosAny) -> RequestMsg {
    assert_eq!(
        request.type_url,
        RequestMsg::TYPE_URL,
        "Different type URL than expected one encountered!"
    );

    RequestMsg::decode(request.value.as_slice()).expect("Expected a swap request message!")
}
