use astroport::{
    asset::AssetInfo,
    router::{ExecuteMsg, SwapOperation, SwapResponseData},
};

use currency::{self, DexSymbols, Group, GroupVisit, SymbolSlice, Tickers};
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
        Any,
    },
    cosmwasm_std::{self, Coin as CwCoin, Decimal},
};

use crate::error::{Error, Result};

use super::{ExactAmountIn, TypeUrl};

pub type RequestMsg = MsgExecuteContract;

impl TypeUrl for RequestMsg {
    const TYPE_URL: &'static str = <Self as sdk::cosmos_sdk_proto::traits::Name>::NAME;
}

pub type ResponseMsg = MsgExecuteContractResponse;

impl TypeUrl for ResponseMsg {
    const TYPE_URL: &'static str = <Self as sdk::cosmos_sdk_proto::traits::Name>::NAME;
}

pub(super) struct Impl;

trait RouterAddress {
    const ROUTER_ADDR: &'static str;
}

// TODO pass the ExactAmountIn impl as a type parameter of the dex package
// #[cfg(any(feature = "net_dev", feature = "net_test"))]
// impl RouterAddress for Impl {
//     /// Source: https://github.com/astroport-fi/astroport-changelog/blob/main/neutron/pion-1/core_testnet.json
//     const ROUTER_ADDR: &'static str =
//         "neutron12jm24l9lr9cupufqjuxpdjnnweana4h66tsx5cl800mke26td26sq7m05p";
// }

// TODO pass the ExactAmountIn impl as a type parameter of the dex package
// temporarily use the main net address
// #[cfg(feature = "net_main")]
impl RouterAddress for Impl {
    /// Source: https://github.com/astroport-fi/astroport-changelog/blob/main/neutron/neutron-1/core_mainnet.json
    const ROUTER_ADDR: &'static str =
        "neutron1rwj6mfxzzrwskur73v326xwuff52vygqk73lr7azkehnfzz5f5wskwekf4";
}

impl ExactAmountIn for Impl {
    fn build<GIn, GSwap>(
        &self,
        trx: &mut Transaction,
        sender: HostAccount,
        token_in: &CoinDTO<GIn>,
        swap_path: &SwapPath,
    ) -> Result<()>
    where
        GIn: Group,
        GSwap: Group,
    {
        const MAX_IMPACT: Decimal = Decimal::percent(50); // 50% is the value of `astroport::pair::MAX_ALLOWED_SLIPPAGE`

        debug_assert!(!swap_path.is_empty());
        let token_in = to_proto_coin(token_in)?;

        to_operations::<GSwap>(&token_in.denom, swap_path)
            .map(|operations| ExecuteMsg::ExecuteSwapOperations {
                operations,
                minimum_receive: None, // disable checks on the received amount
                to: None,              // means the sender
                max_spread: Some(MAX_IMPACT), // if None that would be equivalent to `astroport::pair::DEFAULT_SLIPPAGE`, i.e. 0.5%
            })
            .and_then(|swap_msg| cosmwasm_std::to_json_vec(&swap_msg).map_err(Into::into))
            .map(|msg| RequestMsg {
                sender: sender.into(),
                contract: Self::ROUTER_ADDR.into(),
                msg,
                funds: vec![token_in],
            })
            .map(|req| {
                trx.add_message(RequestMsg::TYPE_URL, req);
            })
    }

    fn parse<I>(&self, trx_resps: &mut I) -> Result<Amount>
    where
        I: Iterator<Item = Any>,
    {
        trx_resps
            .next()
            .ok_or_else(|| Error::MissingResponse("router swap".into()))
            .and_then(|resp| {
                trx::decode_msg_response::<_, ResponseMsg>(resp, ResponseMsg::TYPE_URL)
                    .map_err(Into::into)
            })
            .and_then(|cosmwasm_resp| {
                cosmwasm_std::from_json::<SwapResponseData>(cosmwasm_resp.data).map_err(Into::into)
            })
            .map(|swap_resp| swap_resp.return_amount.into())
    }

    #[cfg(any(test, feature = "testing"))]
    fn build_resp(&self, amount_out: Amount) -> Any {
        use sdk::cosmos_sdk_proto::traits::Message as _;

        let swap_resp = cosmwasm_std::to_json_vec(&SwapResponseData {
            return_amount: amount_out.into(),
        })
        .expect("test result serialization works");
        Any {
            type_url: ResponseMsg::TYPE_URL.into(),
            value: (ResponseMsg { data: swap_resp }).encode_to_vec(),
        }
    }
}

fn to_operations<'a, G>(
    token_in_denom: &'a SymbolSlice,
    swap_path: &'a [SwapTarget],
) -> Result<Vec<SwapOperation>>
where
    G: Group,
{
    struct OperationScan<'a> {
        last_denom: &'a SymbolSlice,
    }

    let scanner = OperationScan {
        last_denom: token_in_denom,
    };

    swap_path
        .iter()
        .map(|swap_target| &swap_target.target)
        .map(to_dex_symbol::<_, G>)
        .scan(scanner, |scanner, may_next_denom| {
            Some(may_next_denom.map(|next_denom| {
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
            }))
        })
        .collect()
}

fn to_proto_coin<G>(token: &CoinDTO<G>) -> Result<ProtoCoin>
where
    G: Group,
{
    coin_legacy::to_cosmwasm_on_network::<G, DexSymbols>(token)
        .map_err(Error::from)
        .map(|CwCoin { denom, amount }| ProtoCoin {
            denom,
            amount: amount.into(),
        })
}

fn to_dex_symbol<Symbol, G>(ticker: &Symbol) -> Result<&SymbolSlice>
where
    Symbol: AsRef<SymbolSlice> + ?Sized,
    G: Group,
{
    Tickers
        .visit_any::<G, _>(ticker.as_ref(), DexSymbols {})
        .map_err(Error::from)
}

#[cfg(test)]
mod test {
    use astroport::{asset::AssetInfo, router::SwapOperation};
    use currency::{
        test::{SubGroupTestC1, SuperGroup, SuperGroupTestC1, SuperGroupTestC6},
        Currency as _, SymbolStatic,
    };
    use finance::coin::Coin;
    use sdk::cosmos_sdk_proto::cosmos::base::v1beta1::Coin as ProtoCoin;

    use crate::error::Error;

    use super::SwapTarget;

    const INVALID_TICKER: SymbolStatic = "NotATicker";

    #[test]
    fn to_dex_symbol() {
        type Currency = SuperGroupTestC1;
        assert_eq!(
            Ok(Currency::DEX_SYMBOL),
            super::to_dex_symbol::<_, SuperGroup>(Currency::TICKER)
        );
    }

    #[test]
    fn to_dex_symbol_err() {
        assert!(matches!(
            super::to_dex_symbol::<_, SuperGroup>(INVALID_TICKER),
            Err(Error::Currency(_))
        ));
    }

    #[test]
    fn to_cwcoin() {
        let coin_amount = 3541415;
        let coin: Coin<SuperGroupTestC1> = coin_amount.into();
        assert_eq!(
            ProtoCoin {
                denom: SuperGroupTestC1::DEX_SYMBOL.into(),
                amount: coin_amount.to_string()
            },
            super::to_proto_coin::<SuperGroup>(&coin.into()).unwrap()
        );
    }

    #[test]
    fn to_operations() {
        type StartSwapCurrency = SubGroupTestC1;
        let path = vec![
            SwapTarget {
                pool_id: 2,
                target: SuperGroupTestC1::TICKER.into(),
            },
            SwapTarget {
                pool_id: 12,
                target: SuperGroupTestC6::TICKER.into(),
            },
        ];
        let expected = vec![
            SwapOperation::AstroSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: StartSwapCurrency::DEX_SYMBOL.into(),
                },
                ask_asset_info: AssetInfo::NativeToken {
                    denom: SuperGroupTestC1::DEX_SYMBOL.into(),
                },
            },
            SwapOperation::AstroSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: SuperGroupTestC1::DEX_SYMBOL.into(),
                },
                ask_asset_info: AssetInfo::NativeToken {
                    denom: SuperGroupTestC6::DEX_SYMBOL.into(),
                },
            },
        ];
        assert_eq!(
            Ok(vec![]),
            super::to_operations::<SuperGroup>(StartSwapCurrency::DEX_SYMBOL, &path[0..0])
        );
        assert_eq!(
            Ok(expected[0..1].to_vec()),
            super::to_operations::<SuperGroup>(StartSwapCurrency::DEX_SYMBOL, &path[0..1])
        );
        assert_eq!(
            Ok(expected),
            super::to_operations::<SuperGroup>(StartSwapCurrency::DEX_SYMBOL, &path)
        );
    }

    #[test]
    fn to_operations_err() {
        let path = vec![SwapTarget {
            pool_id: 2,
            target: INVALID_TICKER.into(),
        }];
        assert!(matches!(
            super::to_operations::<SuperGroup>(SuperGroupTestC1::DEX_SYMBOL, &path),
            Err(Error::Currency(_))
        ));
    }

    #[test]
    fn resp() {
        use crate::trx::ExactAmountIn;

        let amount = 20;
        let instance = super::Impl {};
        let mut resp = vec![instance.build_resp(amount)].into_iter();
        let parsed = instance.parse(&mut resp).unwrap();
        assert_eq!(amount, parsed);
        assert_eq!(None, resp.next());
    }
}
