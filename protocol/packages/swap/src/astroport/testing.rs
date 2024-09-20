use std::any::type_name;

use astroport::{
    asset::AssetInfo,
    router::{ExecuteMsg, SwapOperation, SwapResponseData},
};

use currency::{CurrencyDTO, Group, MemberOf};
use finance::coin::{Amount, CoinDTO};
use oracle::api::swap::{SwapPath, SwapTarget};
use sdk::{
    cosmos_sdk_proto::{
        cosmos::base::v1beta1::Coin as ProtoCoin,
        prost::{Message as _, Name as _},
        Any as CosmosAny,
    },
    cosmwasm_std,
};

use crate::testing::{self, ExactAmountInSkel, SwapRequest};

use super::{Impl, RequestMsg, ResponseMsg, Router};

impl ExactAmountInSkel for Impl {
    fn parse_request<GIn, GSwap>(request: CosmosAny) -> SwapRequest<GIn, GSwap>
    where
        GIn: Group + MemberOf<GSwap>,
        GSwap: Group,
    {
        let RequestMsg {
            sender: _,
            contract,
            msg,
            funds,
        } = parse_request_from_any(request);

        assert_eq!(
            contract,
            <Self as Router>::ADDRESS,
            "Expected message to be addressed to currently selected router!"
        );

        let token_in = parse_one_token_from_vec::<GIn>(funds);

        let ExecuteMsg::ExecuteSwapOperations {
            operations,
            minimum_receive: None,
            to: None,
            max_spread: Some(Self::MAX_IMPACT),
        } = cosmwasm_std::from_json(msg).unwrap_or_else(|_| {
            panic!(
                r#"Expected message to be from type "{}""#,
                type_name::<ExecuteMsg>()
            )
        })
        else {
            testing::pattern_match_else(type_name::<RequestMsg>())
        };

        let swap_path =
            collect_swap_path::<GSwap>(operations, token_in.currency().into_super_group());

        SwapRequest {
            token_in,
            swap_path,
        }
    }

    fn build_response(amount_out: Amount) -> CosmosAny {
        let swap_resp = cosmwasm_std::to_json_vec(&SwapResponseData {
            return_amount: amount_out.into(),
        })
        .expect("test result serialization works");

        CosmosAny {
            type_url: ResponseMsg::type_url(),
            value: (ResponseMsg { data: swap_resp }).encode_to_vec(),
        }
    }
}

fn collect_swap_path<GSwap>(
    operations: Vec<SwapOperation>,
    expected_first_currency: CurrencyDTO<GSwap>,
) -> SwapPath<GSwap>
where
    GSwap: Group,
{
    operations
        .into_iter()
        .scan(
            expected_first_currency,
            |expected_offer_currency, operation| {
                let SwapOperation::AstroSwap {
                    offer_asset_info:
                        AssetInfo::NativeToken {
                            denom: offer_dex_denom,
                        },
                    ask_asset_info:
                        AssetInfo::NativeToken {
                            denom: ask_dex_denom,
                        },
                } = operation
                else {
                    unimplemented!(
                        r#"Expected "AstroSwap" operation with both assets being native tokens!"#
                    );
                };

                let offer_currency = testing::from_dex_symbol::<GSwap>(&offer_dex_denom)
                    .expect("Offered asset doesn't belong to swapping currency group!");
                let ask_currency = testing::from_dex_symbol::<GSwap>(&ask_dex_denom)
                    .expect("Asked asset doesn't belong to swapping currency group!")
                    .to_owned();

                assert_eq!(
                    offer_currency, *expected_offer_currency,
                    "Expected operation's offered denom to be the same as the last asked denom!"
                );

                *expected_offer_currency = ask_currency;

                Some(SwapTarget {
                    pool_id: Default::default(),
                    target: ask_currency,
                })
            },
        )
        .collect()
}

fn parse_request_from_any(request: CosmosAny) -> RequestMsg {
    request.to_msg().expect("Expected a swap request message!")
}

fn parse_one_token_from_vec<G>(funds: Vec<ProtoCoin>) -> CoinDTO<G>
where
    G: Group,
{
    if let [token_in] = funds.as_slice() {
        crate::testing::parse_dex_token(&token_in.amount, &token_in.denom)
    } else {
        unimplemented!("Expected only one type of token!");
    }
}
