use osmosis_std::types::osmosis::poolmanager::v1beta1::{
    MsgSwapExactAmountIn, MsgSwapExactAmountInResponse, SwapAmountInRoute,
};

use currency::{self, DexSymbols, Group, GroupVisit, SymbolSlice, Tickers};
use finance::coin::{Amount, CoinDTO};
use platform::{
    coin_legacy,
    ica::HostAccount,
    trx::{self, Transaction},
};
use sdk::{cosmos_sdk_proto::cosmos::base::abci::v1beta1::MsgData, cosmwasm_std::Coin as CwCoin};

use crate::{
    error::{Error, Result},
    SwapGroup, SwapPath, SwapTarget,
};

#[cfg(not(any(test, feature = "testing")))]
type RequestMsg = MsgSwapExactAmountIn;
#[cfg(any(test, feature = "testing"))]
pub type RequestMsg = MsgSwapExactAmountIn;

#[cfg(not(any(test, feature = "testing")))]
type ResponseMsg = MsgSwapExactAmountInResponse;
#[cfg(any(test, feature = "testing"))]
pub type ResponseMsg = MsgSwapExactAmountInResponse;

pub fn exact_amount_in<G>(
    trx: &mut Transaction,
    sender: HostAccount,
    token_in: &CoinDTO<G>,
    swap_path: &SwapPath,
) -> Result<()>
where
    G: Group,
{
    // TODO bring the token balances, weights and swapFee-s from the DEX pools
    // into the oracle in order to calculate the tokenOut as per the formula at
    // https://docs.osmosis.zone/osmosis-core/modules/gamm/#swap.
    // Then apply the parameterized maximum slippage to get the minimum amount.
    // For the first version, we accept whatever price impact and slippage.
    const MIN_OUT_AMOUNT: &str = "1";
    let routes = to_route(swap_path)?;
    let token_in = Some(to_cwcoin(token_in)?);
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

pub fn exact_amount_in_resp<I>(trx_resps: &mut I) -> Result<Amount>
where
    I: Iterator<Item = MsgData>,
{
    use std::str::FromStr;

    let resp = trx_resps
        .next()
        .ok_or_else(|| Error::MissingResponse("swap of exact amount request".into()))?;

    let amount =
        trx::decode_msg_response::<_, ResponseMsg>(resp, RequestMsg::TYPE_URL)?.token_out_amount;

    Amount::from_str(&amount).map_err(|_| Error::InvalidAmount(amount))
}

#[cfg(any(test, feature = "testing"))]
pub fn build_exact_amount_in_resp(amount_out: Amount) -> MsgData {
    use sdk::cosmos_sdk_proto::traits::Message as _;

    let resp = ResponseMsg {
        token_out_amount: amount_out.to_string(),
    };
    MsgData {
        msg_type: RequestMsg::TYPE_URL.into(),
        data: resp.encode_to_vec(),
    }
}

fn to_route(swap_path: &[SwapTarget]) -> Result<Vec<SwapAmountInRoute>> {
    swap_path
        .iter()
        .map(|swap_target| {
            to_dex_symbol(&swap_target.target).map(|dex_symbol| SwapAmountInRoute {
                pool_id: swap_target.pool_id,
                token_out_denom: dex_symbol.into(),
            })
        })
        .collect()
}

fn to_cwcoin<G>(token: &CoinDTO<G>) -> Result<CwCoin>
where
    G: Group,
{
    coin_legacy::to_cosmwasm_on_network::<G, DexSymbols>(token).map_err(Error::from)
}

fn to_dex_symbol(ticker: &SymbolSlice) -> Result<&SymbolSlice> {
    Tickers
        .visit_any::<SwapGroup, _>(ticker, DexSymbols {})
        .map_err(Error::from)
}

#[cfg(test)]
mod test {
    use currency::{
        lpn::{Lpns, Usdc},
        Currency as _, SymbolStatic,
    };
    use finance::coin::Coin;
    use sdk::cosmwasm_std::Coin as CwCoin;

    use crate::error::Error;

    use super::{SwapAmountInRoute, SwapTarget};

    const INVALID_TICKER: SymbolStatic = "NotATicker";

    #[test]
    fn to_dex_symbol() {
        type Currency = Usdc;
        assert_eq!(
            Ok(Currency::DEX_SYMBOL),
            super::to_dex_symbol(Currency::TICKER)
        );
    }

    #[test]
    fn to_dex_symbol_err() {
        assert!(matches!(
            super::to_dex_symbol(INVALID_TICKER),
            Err(Error::Currency(_))
        ));
    }

    #[test]
    fn to_cwcoin() {
        let coin: Coin<Usdc> = 3541415.into();
        assert_eq!(
            CwCoin::new(coin.into(), Usdc::DEX_SYMBOL),
            super::to_cwcoin::<Lpns>(&coin.into()).unwrap()
        );
    }

    #[test]
    fn into_route() {
        let path = vec![SwapTarget {
            pool_id: 2,
            target: Usdc::TICKER.into(),
        }];
        let expected = vec![SwapAmountInRoute {
            pool_id: 2,
            token_out_denom: Usdc::DEX_SYMBOL.into(),
        }];
        assert_eq!(Ok(expected), super::to_route(&path));
    }

    #[test]
    fn into_route_err() {
        let path = vec![SwapTarget {
            pool_id: 2,
            target: INVALID_TICKER.into(),
        }];
        assert!(matches!(super::to_route(&path), Err(Error::Currency(_))));
    }

    #[test]
    fn resp() {
        let amount = 20;
        let mut resp = vec![super::build_exact_amount_in_resp(amount)].into_iter();
        let parsed = super::exact_amount_in_resp(&mut resp).unwrap();
        assert_eq!(amount, parsed);
        assert_eq!(None, resp.next());
    }
}
