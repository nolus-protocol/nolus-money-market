use std::marker::PhantomData;

use currency::{self, DexSymbols, Group};
use dex::swap::{Error, ExactAmountIn, Result};
use finance::coin::{Amount, CoinDTO};
use oracle::api::swap::{SwapPath, SwapTarget};
use platform::{
    coin_legacy,
    ica::HostAccount,
    trx::{self, Transaction},
};
use sdk::{
    cosmos_sdk_proto::{
        cosmos::base::v1beta1::Coin as ProtoCoin,
        cosmwasm::wasm::v1::{MsgExecuteContract, MsgExecuteContractResponse},
        traits::Name,
        Any as CosmosAny,
    },
    cosmwasm_std::{self, Coin as CwCoin, Decimal},
};

use self::api::{AssetInfo, ExecuteMsg, SwapOperation, SwapResponseData};

mod api;
#[cfg(test)]
mod test;
#[cfg(any(test, feature = "testing"))]
mod testing;

type RequestMsg = MsgExecuteContract;
type ResponseMsg = MsgExecuteContractResponse;

// 50% is the value of `astroport::pair::MAX_ALLOWED_SLIPPAGE`
const MAX_IMPACT: Decimal = Decimal::percent(50);

pub struct Impl<R>
where
    Self: ExactAmountIn,
    R: Router,
{
    _router: PhantomData<R>,
    _never: Never,
}

impl<R> ExactAmountIn for Impl<R>
where
    R: Router,
{
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
        debug_assert!(!swap_path.is_empty());
        let token_in = to_dex_proto_coin(token_in)?;

        cosmwasm_std::to_json_vec(&ExecuteMsg::ExecuteSwapOperations {
            operations: to_operations::<GSwap>(&token_in.denom, swap_path),
            minimum_receive: None, // disable checks on the received amount
            to: None,              // means the sender
            max_spread: Some(MAX_IMPACT), // if None that would be equivalent to `astroport::pair::DEFAULT_SLIPPAGE`, i.e. 0.5%
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

pub trait Router {
    const ADDRESS: &'static str;
}

pub struct NeutronMain {}

impl Router for NeutronMain {
    /// Source: https://github.com/astroport-fi/astroport-changelog/blob/main/neutron/neutron-1/core_mainnet.json
    const ADDRESS: &'static str =
        "neutron1rwj6mfxzzrwskur73v326xwuff52vygqk73lr7azkehnfzz5f5wskwekf4";
}

pub struct NeutronTest {}

impl Router for NeutronTest {
    /// Source: https://github.com/astroport-fi/astroport-changelog/blob/main/neutron/pion-1/core_testnet.json
    const ADDRESS: &'static str =
        "neutron12jm24l9lr9cupufqjuxpdjnnweana4h66tsx5cl800mke26td26sq7m05p";
}

enum Never {}

fn to_operations<'a, G>(
    token_in_denom: &'a str,
    swap_path: &'a [SwapTarget<G>],
) -> Vec<SwapOperation>
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
