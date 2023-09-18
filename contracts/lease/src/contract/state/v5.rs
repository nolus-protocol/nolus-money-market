use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize, Serializer};

use crate::error::ContractResult;

use super::{
    dex::State as DexState, opening, BuyAsset, BuyLpn, Closed, ClosingTransferIn, Liquidated,
    OpenedActive, PaidActive, RequestLoan, Response, SellAsset, SwapResult,
};

type OpenIcaAccount =
    DexState<::dex::IcaConnector<super::opening::open_ica::OpenIcaAccount, SwapResult>>;

#[enum_dispatch]
pub(crate) trait Migrate
where
    Self: Sized,
{
    fn into_last_version(self) -> ContractResult<Response>;
}

#[enum_dispatch(Migrate)]
#[derive(Deserialize)]
pub(in crate::contract) enum StateV4 {
    RequestLoan,
    OpenIcaAccount,
    BuyAsset,
    OpenedActive,
    BuyLpn,
    SellAsset,
    PaidActive,
    ClosingTransferIn,
    Closed,
    Liquidated,
}

impl Migrate for RequestLoan {
    fn into_last_version(self) -> ContractResult<Response> {
        Ok(Response::no_msgs(self))
    }
}

impl Migrate for OpenIcaAccount {
    fn into_last_version(self) -> ContractResult<Response> {
        let next_state = self.map::<_, opening::buy_asset::DexState>(Into::into);
        Ok(Response::no_msgs(next_state))
    }
}

impl Migrate for BuyAsset {
    fn into_last_version(self) -> ContractResult<Response> {
        Ok(Response::no_msgs(self))
    }
}

impl Migrate for OpenedActive {
    fn into_last_version(self) -> ContractResult<Response> {
        Ok(Response::no_msgs(self))
    }
}

impl Migrate for BuyLpn {
    fn into_last_version(self) -> ContractResult<Response> {
        Ok(Response::no_msgs(self))
    }
}
impl Migrate for SellAsset {
    fn into_last_version(self) -> ContractResult<Response> {
        Ok(Response::no_msgs(self))
    }
}
impl Migrate for PaidActive {
    fn into_last_version(self) -> ContractResult<Response> {
        Ok(Response::no_msgs(self))
    }
}
impl Migrate for ClosingTransferIn {
    fn into_last_version(self) -> ContractResult<Response> {
        Ok(Response::no_msgs(self))
    }
}
impl Migrate for Closed {
    fn into_last_version(self) -> ContractResult<Response> {
        Ok(Response::no_msgs(self))
    }
}
impl Migrate for Liquidated {
    fn into_last_version(self) -> ContractResult<Response> {
        Ok(Response::no_msgs(self))
    }
}

impl Serialize for StateV4 {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        unreachable!(
            "Not intended for real use. Required by cw_storage_plus::Item::load trait bounds."
        );
    }
}
