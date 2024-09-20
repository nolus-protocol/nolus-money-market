use std::any::type_name;

use prost::Message;
use serde::{Deserialize, Serialize};

use currency::{DexSymbols, Group};
use dex::swap::{Error, ExactAmountIn, Result};
use finance::coin::{Amount, CoinDTO};
use oracle::api::swap::{SwapPath, SwapTarget};
use platform::{coin_legacy, ica::HostAccount, trx, trx::Transaction};
use sdk::{
    cosmos_sdk_proto::{cosmos::base::v1beta1::Coin as ProtobufCoin, Any as CosmosAny},
    cosmwasm_std::Coin as CosmosCoin,
};

use crate::testing::{self, pattern_match_else, ExactAmountInSkel, SwapRequest};

#[derive(Serialize, Deserialize)]
pub enum Impl {}

const REQUEST_TYPE_URL: &str = "/testing.RequestMsg";
const RESPONSE_TYPE_URL: &str = "/testing.ResponseMsg";

impl ExactAmountIn for Impl {
    fn build_request<GIn, GSwap>(
        trx: &mut Transaction,
        sender: HostAccount,
        token_in: &CoinDTO<GIn>,
        swap_path: &SwapPath<GSwap>,
    ) -> Result<()>
    where
        GIn: Group,
        GSwap: Group,
    {
        trx.add_message(
            REQUEST_TYPE_URL,
            RequestMsg {
                sender: sender.into(),
                token_in: coin_legacy::to_cosmwasm_on_network::<DexSymbols<GIn>>(token_in).map(
                    |CosmosCoin { amount, denom }| {
                        Some(ProtobufCoin {
                            amount: amount.into(),
                            denom,
                        })
                    },
                )?,
                path: swap_path
                    .iter()
                    .map(|swap_target| ProtobufSwapTarget {
                        pool_id: swap_target.pool_id,
                        denom: swap_target.target.into_symbol::<DexSymbols<GSwap>>().into(),
                    })
                    .collect(),
            },
        );

        Ok(())
    }

    fn parse_response(response: CosmosAny) -> Result<Amount> {
        let amount: String = trx::decode_msg_response(response, RESPONSE_TYPE_URL)?;

        amount.parse().map_err(|_| Error::InvalidAmount(amount))
    }
}

impl ExactAmountInSkel for Impl {
    fn parse_request<GIn, GSwap>(request: CosmosAny) -> SwapRequest<GIn, GSwap>
    where
        GIn: Group,
        GSwap: Group,
    {
        let RequestMsg {
            sender: _,
            token_in:
                Some(ProtobufCoin {
                    ref amount,
                    ref denom,
                }),
            path,
        } = parse_request_from_any_and_type_url(request)
        else {
            pattern_match_else(type_name::<RequestMsg>())
        };

        let token_in = testing::parse_dex_token(amount, denom);

        SwapRequest {
            token_in,
            swap_path: path
                .into_iter()
                .map(|ProtobufSwapTarget { pool_id, denom }| SwapTarget {
                    pool_id,
                    target: testing::from_dex_symbol(&denom)
                        .expect("Asked asset doesn't belong to swapping currency group!"),
                })
                .collect(),
        }
    }

    fn build_response(amount_out: Amount) -> CosmosAny {
        CosmosAny {
            type_url: RESPONSE_TYPE_URL.into(),
            value: <ResponseMsg as Message>::encode_to_vec(&amount_out.to_string()),
        }
    }
}

fn parse_request_from_any_and_type_url(request: CosmosAny) -> RequestMsg {
    assert_eq!(
        request.type_url, REQUEST_TYPE_URL,
        "Different type URL than expected one encountered!"
    );

    Message::decode(request.value.as_slice()).expect("Expected a swap request message!")
}

#[derive(Message)]
struct ProtobufSwapTarget {
    #[prost(uint64, tag = 1)]
    pool_id: u64,
    #[prost(string, tag = 2)]
    denom: String,
}

#[derive(Message)]
struct RequestMsg {
    #[prost(string, tag = 1)]
    sender: String,
    #[prost(message, tag = 2)]
    token_in: Option<ProtobufCoin>,
    #[prost(message, repeated, tag = 3)]
    path: Vec<ProtobufSwapTarget>,
}

type ResponseMsg = String;
