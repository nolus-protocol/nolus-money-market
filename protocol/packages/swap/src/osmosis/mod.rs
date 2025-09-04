use osmosis_std::types::osmosis::poolmanager::v1beta1::{
    MsgSwapExactAmountIn, MsgSwapExactAmountInResponse, SwapAmountInRoute,
};

use currency::{DexSymbols, Group};
use dex::swap::{Error, ExactAmountIn, Result, SwapPathSlice};
use finance::coin::{Amount, CoinDTO};
use oracle::api::swap::SwapTarget;
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
    fn build_request<GIn, GOut, GSwap>(
        trx: &mut Transaction,
        sender: HostAccount,
        amount_in: &CoinDTO<GIn>,
        min_amount_out: &CoinDTO<GOut>,
        swap_path: SwapPathSlice<'_, GSwap>,
    ) -> Result<()>
    where
        GIn: Group,
        GOut: Group,
        GSwap: Group,
    {
        // TODO bring the token balances, weights and swapFee-s from the DEX pools
        // into the oracle in order to calculate the tokenOut as per the formula at
        // https://docs.osmosis.zone/osmosis-core/modules/gamm/#swap.
        // Then apply the parameterized maximum slippage to get the minimum amount.
        // For the first version, we accept whatever price impact and slippage.
        let routes = to_route::<GSwap>(swap_path);
        let token_in = Some(to_dex_cwcoin(amount_in));
        let token_out_min_amount = min_amount_out.amount().to_string();
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
        trx_resps
            .next()
            .ok_or_else(|| Error::MissingResponse("swap of exact amount request".into()))
            .and_then(|response| {
                trx::decode_msg_response::<_, ResponseMsg>(response, ResponseMsg::TYPE_URL)
                    .map_err(Into::into)
            })
            .map(|response| response.token_out_amount)
            .and_then(|amount| amount.parse().map_err(|_| Error::InvalidAmount(amount)))
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

fn to_dex_cwcoin<G>(token: &CoinDTO<G>) -> CwCoin
where
    G: Group,
{
    coin_legacy::to_cosmwasm_on_network::<DexSymbols<G>>(token)
}
