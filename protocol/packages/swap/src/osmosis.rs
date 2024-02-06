#[cfg(feature = "testing")]
use std::any::type_name;

use osmosis_std::types::osmosis::poolmanager::v1beta1::{
    MsgSwapExactAmountIn, MsgSwapExactAmountInResponse, SwapAmountInRoute,
};
use serde::{Deserialize, Serialize};

use currency::{DexSymbols, Group, GroupVisit, SymbolSlice, Tickers};
#[cfg(feature = "testing")]
use dex::swap::SwapRequest;
use dex::swap::{Error, ExactAmountIn, Result};
use finance::coin::{Amount, CoinDTO};
use oracle::api::swap::{SwapPath, SwapTarget};
use platform::{
    coin_legacy,
    ica::HostAccount,
    trx::{self, Transaction},
};
#[cfg(feature = "testing")]
use sdk::cosmos_sdk_proto::prost::Message;
use sdk::{cosmos_sdk_proto::Any, cosmwasm_std::Coin as CwCoin};

// TODO change visibility to private
pub type RequestMsg = MsgSwapExactAmountIn;
type ResponseMsg = MsgSwapExactAmountInResponse;

#[derive(Serialize, Deserialize)]
pub struct Impl;

impl ExactAmountIn for Impl {
    fn build_request<GIn, GSwap>(
        trx: &mut Transaction,
        sender: HostAccount,
        token_in: &CoinDTO<GIn>,
        swap_path: &SwapPath,
    ) -> Result<()>
    where
        GIn: Group,
        GSwap: Group,
    {
        // TODO bring the token balances, weights and swapFee-s from the DEX pools
        // into the oracle in order to calculate the tokenOut as per the formula at
        // https://docs.osmosis.zone/osmosis-core/modules/gamm/#swap.
        // Then apply the parameterized maximum slippage to get the minimum amount.
        // For the first version, we accept whatever price impact and slippage.
        const MIN_OUT_AMOUNT: &str = "1";
        let routes = to_route::<GSwap>(swap_path)?;
        let token_in = Some(to_dex_cwcoin(token_in)?);
        let token_out_min_amount = MIN_OUT_AMOUNT.into();
        let msg = RequestMsg {
            sender: sender.into(),
            routes,
            token_in: token_in.map(Into::into),
            token_out_min_amount,
        };

        trx.add_message(RequestMsg::TYPE_URL, msg);

        Ok(())
    }

    fn parse_response<I>(trx_resps: &mut I) -> Result<Amount>
    where
        I: Iterator<Item = Any>,
    {
        use std::str::FromStr;

        let resp = trx_resps
            .next()
            .ok_or_else(|| Error::MissingResponse("swap of exact amount request".into()))?;

        let amount = trx::decode_msg_response::<_, ResponseMsg>(resp, ResponseMsg::TYPE_URL)?
            .token_out_amount;

        Amount::from_str(&amount).map_err(|_| Error::InvalidAmount(amount))
    }

    #[cfg(feature = "testing")]
    fn parse_request<GIn, GSwap>(request: Any) -> SwapRequest<GIn>
    where
        GIn: Group,
        GSwap: Group,
    {
        let RequestMsg {
            sender: _,
            routes,
            token_in: Some(token_in),
            token_out_min_amount,
        } = parse_request_from_any_and_type_url(request, RequestMsg::TYPE_URL)
        else {
            crate::pattern_match_else(type_name::<RequestMsg>())
        };

        assert_eq!({ token_out_min_amount }, "1");

        let token_in = crate::parse_dex_token(&token_in.amount, token_in.denom);

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
                            target: currency::DexSymbols
                                .visit_any::<GSwap, _>(&target, currency::Tickers)
                                .expect("Asked asset doesn't belong to swapping currency group!")
                                .into(),
                        }
                    },
                )
                .collect(),
        }
    }

    #[cfg(feature = "testing")]
    fn build_response(amount_out: Amount) -> Any {
        use sdk::cosmos_sdk_proto::traits::Message as _;

        let resp = ResponseMsg {
            token_out_amount: amount_out.to_string(),
        };
        Any {
            type_url: ResponseMsg::TYPE_URL.into(),
            value: resp.encode_to_vec(),
        }
    }
}

fn to_route<G>(swap_path: &[SwapTarget]) -> Result<Vec<SwapAmountInRoute>>
where
    G: Group,
{
    swap_path
        .iter()
        .map(|swap_target| {
            to_dex_symbol::<G>(&swap_target.target).map(|dex_symbol| SwapAmountInRoute {
                pool_id: swap_target.pool_id,
                token_out_denom: dex_symbol.into(),
            })
        })
        .collect()
}

fn to_dex_cwcoin<G>(token: &CoinDTO<G>) -> Result<CwCoin>
where
    G: Group,
{
    coin_legacy::to_cosmwasm_on_network::<G, DexSymbols>(token).map_err(Error::from)
}

fn to_dex_symbol<G>(ticker: &SymbolSlice) -> Result<&SymbolSlice>
where
    G: Group,
{
    Tickers
        .visit_any::<G, _>(ticker, DexSymbols {})
        .map_err(Error::from)
}

#[cfg(feature = "testing")]
fn parse_request_from_any_and_type_url<T>(request: Any, type_url: &str) -> T
where
    T: Message + Default,
{
    assert_eq!(
        request.type_url, type_url,
        "Different type URL than expected one encountered!"
    );

    T::decode(request.value.as_slice()).expect("Expected a swap request message!")
}

#[cfg(test)]
mod test {
    use currency::test::{SuperGroup, SuperGroupTestC1};
    use currency::{Currency as _, SymbolStatic};
    use dex::swap::Error;
    use finance::coin::Coin;
    use sdk::cosmwasm_std::Coin as CwCoin;

    use super::{SwapAmountInRoute, SwapTarget};

    const INVALID_TICKER: SymbolStatic = "NotATicker";

    #[test]
    fn to_dex_symbol() {
        type Currency = SuperGroupTestC1;
        assert_eq!(
            Ok(Currency::DEX_SYMBOL),
            super::to_dex_symbol::<SuperGroup>(Currency::TICKER)
        );
    }

    #[test]
    fn to_dex_symbol_err() {
        assert!(matches!(
            super::to_dex_symbol::<SuperGroup>(INVALID_TICKER),
            Err(Error::Currency(_))
        ));
    }

    #[test]
    fn to_dex_cwcoin() {
        let coin: Coin<SuperGroupTestC1> = 3541415.into();
        assert_eq!(
            CwCoin::new(coin.into(), SuperGroupTestC1::DEX_SYMBOL),
            super::to_dex_cwcoin::<SuperGroup>(&coin.into()).unwrap()
        );
    }

    #[test]
    fn into_route() {
        let path = vec![SwapTarget {
            pool_id: 2,
            target: SuperGroupTestC1::TICKER.into(),
        }];
        let expected = vec![SwapAmountInRoute {
            pool_id: 2,
            token_out_denom: SuperGroupTestC1::DEX_SYMBOL.into(),
        }];
        assert_eq!(Ok(expected), super::to_route::<SuperGroup>(&path));
    }

    #[test]
    fn into_route_err() {
        let path = vec![SwapTarget {
            pool_id: 2,
            target: INVALID_TICKER.into(),
        }];
        assert!(matches!(
            super::to_route::<SuperGroup>(&path),
            Err(Error::Currency(_))
        ));
    }
}
