use ::currency::payment::PaymentGroup;
use finance::{
    coin::CoinDTO,
    currency::{self, AnyVisitor, Currency, Symbol, SymbolOwned},
};
use oracle::{msg::SwapPathResponse, state::supported_pairs::SwapTarget};
use osmosis_std::types::{
    cosmos::base::v1beta1::Coin,
    osmosis::gamm::v1beta1::{MsgSwapExactAmountIn, SwapAmountInRoute},
};
use platform::ica::Batch;
use sdk::cosmwasm_std::Addr;

use crate::error::{Error, Result};

pub mod error;

pub fn exact_amount_in(
    batch: &mut Batch,
    sender: &Addr,
    token_in: &CoinDTO,
    swap_path: &SwapPathResponse,
) -> Result<()> {
    const MSG_TYPE: &str = "/osmosis.gamm.v1beta1.MsgSwapExactAmountIn";
    // TODO bring the token balances, weights and swapFee-s from the DEX pools
    // into the oracle in order to calculate the tokenOut as per the formula at
    // https://docs.osmosis.zone/osmosis-core/modules/gamm/#swap.
    // Then apply the parameterized maximum slippage to get the minimum amount.
    // For the first version, we accept whatever price impact and slippage.
    const MIN_OUT_AMOUNT: &str = "0";
    let routes = into_route(swap_path)?;
    let token_in = Some(into_coin(token_in)?);
    let token_out_min_amount = MIN_OUT_AMOUNT.into();
    let msg = MsgSwapExactAmountIn {
        sender: sender.into(),
        routes,
        token_in,
        token_out_min_amount,
    };

    batch.add_message(MSG_TYPE, msg)?;
    Ok(())
}

fn into_route(swap_path: &[SwapTarget]) -> Result<Vec<SwapAmountInRoute>> {
    swap_path
        .iter()
        .map(|swap_target| {
            to_dex_symbol(&swap_target.target).map(|dex_symbol| SwapTarget {
                pool_id: swap_target.pool_id,
                target: dex_symbol,
            })
        })
        .map(|maybe_swap_target| {
            maybe_swap_target.map(|swap_target| SwapAmountInRoute {
                pool_id: swap_target.pool_id,
                token_out_denom: swap_target.target,
            })
        })
        .collect()
}

fn into_coin(token: &CoinDTO) -> Result<Coin> {
    Ok(Coin {
        denom: to_dex_symbol(token.ticker())?,
        amount: token.amount().to_string(),
    })
}

fn to_dex_symbol(ticker: Symbol) -> Result<SymbolOwned> {
    type SwapGroup = PaymentGroup;

    struct ToDEXSymbol {}
    impl AnyVisitor for ToDEXSymbol {
        type Output = SymbolOwned;
        type Error = Error;

        fn on<C>(self) -> Result<Self::Output>
        where
            C: Currency,
        {
            Ok(C::DEX_SYMBOL.into())
        }
    }
    currency::visit_any_on_ticker::<SwapGroup, _>(ticker, ToDEXSymbol {})
}

#[cfg(test)]
mod test {
    use currency::lpn::Usdc;
    use finance::currency::{Currency, SymbolStatic};
    use oracle::state::supported_pairs::SwapTarget;
    use osmosis_std::types::osmosis::gamm::v1beta1::SwapAmountInRoute;

    use crate::error::Error;

    const INVALID_TICKER: SymbolStatic = "NotATicker";

    #[test]
    fn to_dex_symbol() {
        type Currency = Usdc;
        assert_eq!(
            Ok(Currency::DEX_SYMBOL.into()),
            super::to_dex_symbol(Currency::TICKER)
        );
    }

    #[test]
    fn to_dex_symbol_err() {
        assert!(matches!(
            super::to_dex_symbol(INVALID_TICKER),
            Err(Error::Finance(_))
        ));
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
        assert_eq!(Ok(expected), super::into_route(&path));
    }

    #[test]
    fn into_route_err() {
        let path = vec![SwapTarget {
            pool_id: 2,
            target: INVALID_TICKER.into(),
        }];
        assert!(matches!(super::into_route(&path), Err(Error::Finance(_))));
    }
}
