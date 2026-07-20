use std::marker::PhantomData;

use currency::{self, DexSymbols, Group};
use dex::{SwapError, SwapResult, Transport};
use finance::coin::{Amount, CoinDTO};
use platform::{
    coin_legacy,
    remote::Account as HostAccount,
    trx::{self, Transaction},
};
use sdk::{
    api::ProtobufAny,
    cosmos_sdk_proto::{
        cosmos::base::v1beta1::Coin as ProtoCoin,
        cosmwasm::wasm::v1::{MsgExecuteContract, MsgExecuteContractResponse},
        traits::Name,
    },
    cosmwasm_std::{self, Coin as CwCoin, Decimal},
};

use self::{
    api::{ExecuteMsg, SwapResponseData},
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
    Self: Transport,
    R: Router,
{
    _router: PhantomData<R>,
}

impl<R> Default for GenericImpl<R>
where
    R: Router,
{
    fn default() -> Self {
        Self {
            _router: PhantomData,
        }
    }
}

impl<R> Transport for GenericImpl<R>
where
    R: Router,
{
    fn build_request<GIn, GOut>(
        trx: &mut Transaction,
        sender: HostAccount,
        token_in: &CoinDTO<GIn>,
        min_amount_out: &CoinDTO<GOut>,
    ) -> SwapResult<()>
    where
        GIn: Group,
        GOut: Group,
    {
        let token_in = to_dex_proto_coin(token_in);

        cosmwasm_std::to_json_vec(&ExecuteMsg::ExecuteSwapOperations {
            operations: Default::default(),
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

    fn parse_response<I>(trx_resps: &mut I) -> SwapResult<Amount>
    where
        I: Iterator<Item = ProtobufAny>,
    {
        trx_resps
            .next()
            .ok_or_else(|| SwapError::MissingResponse("router swap".into()))
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

fn to_dex_proto_coin<G>(token: &CoinDTO<G>) -> ProtoCoin
where
    G: Group,
{
    let CwCoin { denom, amount } = coin_legacy::to_cosmwasm_on_network::<DexSymbols<G>>(token);
    ProtoCoin {
        denom,
        amount: amount.into(),
    }
}
