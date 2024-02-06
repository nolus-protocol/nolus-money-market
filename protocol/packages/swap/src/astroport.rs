#[cfg(feature = "testing")]
use std::any::type_name;
use std::marker::PhantomData;

use astroport::{
    asset::AssetInfo,
    router::{ExecuteMsg, SwapOperation, SwapResponseData},
};
use serde::{Deserialize, Serialize};

use currency::{self, DexSymbols, Group, GroupVisit, SymbolSlice, Tickers};
use dex::swap::{Error, ExactAmountIn, Result};
#[cfg(feature = "testing")]
use dex::swap::SwapRequest;
use finance::coin::{Amount, CoinDTO};
use oracle::api::swap::{SwapPath, SwapTarget};
use platform::{
    coin_legacy,
    ica::HostAccount,
    trx::{self, Transaction},
};
#[cfg(feature = "testing")]
use sdk::cosmos_sdk_proto::prost::Message;
use sdk::{
    cosmos_sdk_proto::{
        cosmos::base::v1beta1::Coin as ProtoCoin,
        cosmwasm::wasm::v1::{MsgExecuteContract, MsgExecuteContractResponse},
        traits::Name,
        Any,
    },
    cosmwasm_std::{self, Coin as CwCoin, Decimal},
};

// TODO change visibility to private
pub type RequestMsg = MsgExecuteContract;
type ResponseMsg = MsgExecuteContractResponse;

trait Router {
    const ROUTER_ADDR: &'static str;
}

pub struct Main {}

impl Router for Main {
    /// Source: https://github.com/astroport-fi/astroport-changelog/blob/main/neutron/neutron-1/core_mainnet.json
    const ROUTER_ADDR: &'static str =
        "neutron1rwj6mfxzzrwskur73v326xwuff52vygqk73lr7azkehnfzz5f5wskwekf4";
}

pub struct Test {}

impl Router for Test {
    /// Source: https://github.com/astroport-fi/astroport-changelog/blob/main/neutron/pion-1/core_testnet.json
    const ROUTER_ADDR: &'static str =
        "neutron12jm24l9lr9cupufqjuxpdjnnweana4h66tsx5cl800mke26td26sq7m05p";
}

#[derive(Serialize, Deserialize)]
pub struct RouterImpl<R>(PhantomData<R>);

impl<R> RouterImpl<R> {
    const MAX_IMPACT: Decimal = Decimal::percent(50); // 50% is the value of `astroport::pair::MAX_ALLOWED_SLIPPAGE`
}

impl<R> ExactAmountIn for RouterImpl<R>
where
    R: Router,
{
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
        debug_assert!(!swap_path.is_empty());
        let token_in = to_dex_proto_coin(token_in)?;

        to_operations::<GSwap>(&token_in.denom, swap_path)
            .map(|operations| ExecuteMsg::ExecuteSwapOperations {
                operations,
                minimum_receive: None, // disable checks on the received amount
                to: None,              // means the sender
                max_spread: Some(Self::MAX_IMPACT), // if None that would be equivalent to `astroport::pair::DEFAULT_SLIPPAGE`, i.e. 0.5%
            })
            .and_then(|swap_msg| cosmwasm_std::to_json_vec(&swap_msg).map_err(Into::into))
            .map(|msg| RequestMsg {
                sender: sender.into(),
                contract: R::ROUTER_ADDR.into(),
                msg,
                funds: vec![token_in],
            })
            .map(|req| {
                trx.add_message(RequestMsg::NAME, req);
            })
    }

    fn parse_response<I>(trx_resps: &mut I) -> Result<Amount>
    where
        I: Iterator<Item = Any>,
    {
        trx_resps
            .next()
            .ok_or_else(|| Error::MissingResponse("router swap".into()))
            .and_then(|resp| {
                trx::decode_msg_response::<_, ResponseMsg>(resp, ResponseMsg::NAME)
                    .map_err(Into::into)
            })
            .and_then(|cosmwasm_resp| {
                cosmwasm_std::from_json::<SwapResponseData>(cosmwasm_resp.data).map_err(Into::into)
            })
            .map(|swap_resp| swap_resp.return_amount.into())
    }

    #[cfg(feature = "testing")]
    fn parse_request<GIn>(request: Any) -> SwapRequest<GIn>
    where
        GIn: Group,
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
            crate::pattern_match_else("ExecuteSwapOperations");
        };

        let swap_path = collect_swap_path(operations, token_in.ticker().clone());

        SwapRequest {
            token_in,
            swap_path,
        }
    }

    #[cfg(feature = "testing")]
    fn build_response(amount_out: Amount) -> Any {
        use sdk::cosmos_sdk_proto::traits::Message as _;

        let swap_resp = cosmwasm_std::to_json_vec(&SwapResponseData {
            return_amount: amount_out.into(),
        })
        .expect("test result serialization works");
        Any {
            type_url: ResponseMsg::NAME.into(),
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
        .map(|swap_target| swap_target.target.as_str())
        .map(to_dex_symbol::<G>)
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

#[cfg(feature = "testing")]
fn collect_swap_path(operations: Vec<SwapOperation>, token_in: String) -> SwapPath {
    operations
        .into_iter()
        .scan(token_in, |expected_offer_denom, operation| {
            let SwapOperation::AstroSwap {
                offer_asset_info: AssetInfo::NativeToken { denom: offer_denom },
                ask_asset_info: AssetInfo::NativeToken { denom: ask_denom },
            } = operation
            else {
                unimplemented!(
                    r#"Expected "AstroSwap" operation with both assets being native tokens!"#
                );
            };

            assert_eq!(
                offer_denom.as_str(),
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

fn to_dex_proto_coin<G>(token: &CoinDTO<G>) -> Result<ProtoCoin>
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

fn to_dex_symbol<G>(ticker: &SymbolSlice) -> Result<&SymbolSlice>
where
    G: Group,
{
    Tickers
        .visit_any::<G, _>(ticker, DexSymbols {})
        .map_err(Error::from)
}

#[cfg(feature = "testing")]
fn parse_request_from_any<T>(request: Any) -> T
where
    T: Message + Default + Name,
{
    request.to_msg().expect("Expected a swap request message!")
}

#[cfg(feature = "testing")]
fn parse_one_token_from_vec<G>(funds: Vec<ProtoCoin>) -> CoinDTO<G>
where
    G: Group,
{
    if let [token_in] = funds.as_slice() {
        crate::parse_token(&token_in.amount, token_in.denom.clone())
    } else {
        unimplemented!("Expected only one type of token!");
    }
}

#[cfg(test)]
mod test {
    use astroport::{asset::AssetInfo, router::SwapOperation};

    use currency::{
        test::{SubGroupTestC1, SuperGroup, SuperGroupTestC1, SuperGroupTestC6},
        Currency as _, SymbolStatic,
    };
    use dex::swap::Error;
    use finance::coin::Coin;
    use oracle::api::swap::SwapTarget;
    use sdk::{cosmos_sdk_proto::cosmos::base::v1beta1::Coin as ProtoCoin, cosmwasm_std::Decimal};

    use super::{Main, RouterImpl};

    const INVALID_TICKER: SymbolStatic = "NotATicker";

    #[test]
    fn const_eq_max_allowed_slippage() {
        assert_eq!(
            RouterImpl::<Main>::MAX_IMPACT,
            astroport::pair::MAX_ALLOWED_SLIPPAGE
                .parse::<Decimal>()
                .unwrap()
        );
    }

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
        let coin_amount = 3541415;
        let coin: Coin<SuperGroupTestC1> = coin_amount.into();
        assert_eq!(
            ProtoCoin {
                denom: SuperGroupTestC1::DEX_SYMBOL.into(),
                amount: coin_amount.to_string(),
            },
            super::to_dex_proto_coin::<SuperGroup>(&coin.into()).unwrap()
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

    #[cfg(feature = "testing")]
    #[test]
    fn resp() {
        use dex::swap::ExactAmountIn;

        type SwapClient = super::RouterImpl<Main>;

        let amount = 20;
        let mut resp = vec![SwapClient::build_response(amount)].into_iter();
        let parsed = SwapClient::parse_response(&mut resp).unwrap();
        assert_eq!(amount, parsed);
        assert_eq!(None, resp.next());
    }
}
