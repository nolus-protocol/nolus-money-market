use std::marker::PhantomData;

use currency::{self, DexSymbols, Group};
use dex::swap::{Error, ExactAmountIn, Result, SwapPathSlice};
use finance::coin::{Amount, CoinDTO};
use platform::{
    coin_legacy,
    ica::HostAccount,
    trx::{self, Transaction},
};
use sdk::{
    cosmos_sdk_proto::{
        Any as CosmosAny,
        cosmos::base::v1beta1::Coin as ProtoCoin,
        cosmwasm::wasm::v1::{MsgExecuteContract, MsgExecuteContractResponse},
        traits::Name,
    },
    cosmwasm_std::{self, Coin as CwCoin, Decimal},
};

use self::{
    api::{AssetInfo, ExecuteMsg, SwapOperation, SwapResponseData},
    router::Router,
};

pub type Impl = GenericImpl<self::router::Impl>;

mod api;
mod router;
#[cfg(test)]
mod test;
#[cfg(any(test, feature = "testing"))]
mod testing;

type RequestMsg = MsgExecuteContract;
type ResponseMsg = MsgExecuteContractResponse;

// 50% is the value of `astroport::pair::MAX_ALLOWED_SLIPPAGE`
const MAX_IMPACT: Decimal = Decimal::percent(50);

pub struct GenericImpl<R>
where
    Self: ExactAmountIn,
    R: Router,
{
    _router: PhantomData<R>,
    _never: Never,
}

impl<R> ExactAmountIn for GenericImpl<R>
where
    R: Router,
{
    fn build_request<GIn, GOut, GSwap>(
        trx: &mut Transaction,
        sender: HostAccount,
        token_in: &CoinDTO<GIn>,
        min_amount_out: &CoinDTO<GOut>,
        swap_path: SwapPathSlice<'_, GSwap>,
    ) -> Result<()>
    where
        GIn: Group,
        GOut: Group,
        GSwap: Group,
    {
        debug_assert!(!swap_path.is_empty());
        let token_in = to_dex_proto_coin(token_in)?;

        cosmwasm_std::to_json_vec(&ExecuteMsg::ExecuteSwapOperations {
            operations: to_operations::<GSwap>(&token_in.denom, swap_path),
            minimum_receive: Some(min_amount_out.amount().into()), // request a check on the received amount
            to: None,                                              // means the sender
            max_spread: Some(MAX_IMPACT), // checked on each individual swap operation, if None that would be equivalent to `astroport::pair::DEFAULT_SLIPPAGE`, i.e. 0.5%,
                                          // fails if greater-than MAX_IMPACT https://github.com/astroport-fi/astroport-core/blob/b558de92ef4bf8f3dc3a272f2ec45a317eff43bf/contracts/pair/src/contract.rs#L1363C20-L1363C57
        })
        .map_err(Into::into)
        .map(|msg| RequestMsg {
            sender: sender.into(),
            contract: R::ADDRESS.into(),
            msg,
            funds: vec![token_in],
        })
        .map(|req| {
            trx.add_message(RequestMsg::type_url(), req);
        })
    }

    fn parse_response<I>(trx_resps: &mut I) -> Result<Amount>
    where
        I: Iterator<Item = CosmosAny>,
    {
        trx_resps
            .next()
            .ok_or_else(|| Error::MissingResponse("router swap".into()))
            .and_then(|resp| {
                trx::decode_msg_response::<_, ResponseMsg>(resp, ResponseMsg::type_url())
                    .map_err(Into::into)
            })
            .and_then(|cosmwasm_resp| {
                cosmwasm_std::from_json::<SwapResponseData>(cosmwasm_resp.data).map_err(Into::into)
            })
            .map(|swap_resp| swap_resp.return_amount.into())
    }
}

enum Never {}

fn to_operations<G>(token_in_denom: &str, swap_path: SwapPathSlice<'_, G>) -> Vec<SwapOperation>
where
    G: Group,
{
    struct OperationScan<'a> {
        last_denom: &'a str,
    }

    let scanner = OperationScan {
        last_denom: token_in_denom,
    };

    swap_path
        .iter()
        .map(|swap_target| swap_target.target.into_symbol::<DexSymbols<G>>())
        .scan(scanner, |scanner, next_denom| {
            Some({
                let op = SwapOperation::AstroSwap {
                    offer_asset_info: AssetInfo::NativeToken {
                        denom: scanner.last_denom.into(),
                    },
                    ask_asset_info: AssetInfo::NativeToken {
                        denom: next_denom.into(),
                    },
                };
                scanner.last_denom = next_denom;
                op
            })
        })
        .collect()
}

fn to_dex_proto_coin<G>(token: &CoinDTO<G>) -> Result<ProtoCoin>
where
    G: Group,
{
    coin_legacy::to_cosmwasm_on_network::<DexSymbols<G>>(token)
        .map_err(Error::from)
        .map(|CwCoin { denom, amount }| ProtoCoin {
            denom,
            amount: amount.into(),
        })
}
