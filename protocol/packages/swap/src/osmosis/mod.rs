use osmosis_std::types::osmosis::poolmanager::v1beta1::{
    MsgSwapExactAmountIn, MsgSwapExactAmountInResponse, SwapAmountInRoute,
};

use currency::{DexSymbols, Group};
use dex::swap::{Error, ExactAmountIn, Result};
use finance::coin::{Amount, CoinDTO};
use oracle::api::swap::{SwapPath, SwapTarget};
use platform::{
    coin_legacy,
    ica::HostAccount,
    trx::{self, Transaction},
};
use sdk::{cosmos_sdk_proto::Any as CosmosAny, cosmwasm_std::Coin as CwCoin};

#[cfg(test)]
mod test;
#[cfg(any(test, feature = "testing"))]
mod testing;

type RequestMsg = MsgSwapExactAmountIn;
type ResponseMsg = MsgSwapExactAmountInResponse;

pub enum Impl
where
    Self: ExactAmountIn, {}

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
        // TODO bring the token balances, weights and swapFee-s from the DEX pools
        // into the oracle in order to calculate the tokenOut as per the formula at
        // https://docs.osmosis.zone/osmosis-core/modules/gamm/#swap.
        // Then apply the parameterized maximum slippage to get the minimum amount.
        // For the first version, we accept whatever price impact and slippage.
        const MIN_OUT_AMOUNT: &str = "1";
        let routes = to_route::<GSwap>(swap_path);
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
        I: Iterator<Item = CosmosAny>,
    {
        use std::str::FromStr;

        let resp = trx_resps
            .next()
            .ok_or_else(|| Error::MissingResponse("swap of exact amount request".into()))?;

        let amount = trx::decode_msg_response::<_, ResponseMsg>(resp, ResponseMsg::TYPE_URL)?
            .token_out_amount;

        Amount::from_str(&amount).map_err(|_| Error::InvalidAmount(amount))
    }
}

fn to_route<GSwap>(swap_path: &[SwapTarget<GSwap>]) -> Vec<SwapAmountInRoute>
where
    GSwap: Group,
{
    swap_path
        .iter()
        .map(|swap_target| SwapAmountInRoute {
            pool_id: swap_target.pool_id,
            token_out_denom: swap_target.target.into_symbol::<DexSymbols<GSwap>>().into(),
        })
        .collect()
}

fn to_dex_cwcoin<G>(token: &CoinDTO<G>) -> Result<CwCoin>
where
    G: Group,
{
    coin_legacy::to_cosmwasm_on_network::<DexSymbols<G>>(token).map_err(Error::from)
}
