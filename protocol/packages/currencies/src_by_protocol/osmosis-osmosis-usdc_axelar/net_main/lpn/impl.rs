use sdk::schemars;

use currency::InPoolWith;

use crate::{define_currency, lease::impl_mod::Osmo, payment::only::impl_mod::UsdcNoble, Lpns};

define_currency!(
    Usdc,
    "USDC",
    "ibc/7FBDBEEEBA9C50C4BCDF7BF438EAB99E64360833D240B32655C96E319559E911", // transfer/channel-0/transfer/channel-208/uusdc
    "ibc/D189335C6E4A68B513C10AB227BF1C1D38C746766278BA3EEB4FB14124F1D858", // transfer/channel-208/uusdc
    Lpns,
    6
);

pub use Usdc as Lpn;

impl InPoolWith<UsdcNoble> for Lpn {}
impl InPoolWith<Osmo> for Lpn {}
