use std::any::type_name;

use astroport::{
    asset::AssetInfo,
    router::{ExecuteMsg, SwapOperation, SwapResponseData},
};

use currency::{DexSymbols, Group, GroupVisit as _, SymbolSlice, Tickers};
use dex::swap::{Error, ExactAmountIn};
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

use crate::testing::{pattern_match_else, ExactAmountInExt, SwapRequest};

use super::{RequestMsg, ResponseMsg, Router, RouterImpl};

impl<R> ExactAmountInExt for RouterImpl<R>
where
    Self: ExactAmountIn,
    R: Router,
{
    fn parse_request<GIn, GSwap>(request: CosmosAny) -> SwapRequest<GIn>
    where
        GIn: Group,
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
            R::ROUTER_ADDR,
            "Expected message to be addressed to currently selected router!"
        );

        let token_in = parse_one_token_from_vec(funds);

        let ExecuteMsg::ExecuteSwapOperations {
            operations,
            minimum_receive: None,
            to: None,
            max_spread: Some(Self::MAX_IMPACT),
        } = cosmwasm_std::from_json(msg).expect(&format!(
            r#"Expected message to be from type "{}""#,
            type_name::<ExecuteMsg>()
        ))
        else {
            pattern_match_else("ExecuteSwapOperations");
        };

        let swap_path = collect_swap_path::<GSwap>(operations, token_in.ticker().clone());

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

fn collect_swap_path<GSwap>(operations: Vec<SwapOperation>, token_in: String) -> SwapPath
where
    GSwap: Group,
{
    operations
        .into_iter()
        .scan(token_in, |expected_offer_denom, operation| {
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

            let offer_denom = from_dex_symbol::<GSwap>(&offer_dex_denom)
                .expect("Offered asset doesn't belong to swapping currency group!");
            let ask_denom = from_dex_symbol::<GSwap>(&ask_dex_denom)
                .expect("Asked asset doesn't belong to swapping currency group!")
                .to_owned();

            assert_eq!(
                offer_denom,
                expected_offer_denom.as_str(),
                "Expected operation's offered denom to be the same as the last asked denom!"
            );

            *expected_offer_denom = ask_denom.clone();

            Some(SwapTarget {
                pool_id: Default::default(),
                target: ask_denom,
            })
        })
        .collect()
}

fn from_dex_symbol<G>(ticker: &SymbolSlice) -> dex::swap::Result<&SymbolSlice>
where
    G: Group,
{
    DexSymbols
        .visit_any::<G, _>(ticker, Tickers {})
        .map_err(Error::from)
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
