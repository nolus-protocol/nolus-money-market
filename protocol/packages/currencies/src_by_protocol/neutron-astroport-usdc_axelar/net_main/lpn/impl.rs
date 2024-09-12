use currency::InPoolWith;
use sdk::schemars;

use crate::{define_currency, lease::impl_mod::Ntrn, payment::only::impl_mod::UsdcNoble, Lpns};

define_currency!(
    UsdcAxelar,
    "USDC_AXELAR",
    "ibc/076CF690A9912E0B7A2CCA75B719D68AF7C20E4B0B6460569B333DDEB19BBBA1", // transfer/channel-3839/transfer/channel-2/uusdc
    "ibc/F082B65C88E4B6D5EF1DB243CDA1D331D002759E938A0F5CD3FFDC5D53B3E349", // transfer/channel-2/uusdc
    Lpns,
    6
);

pub use UsdcAxelar as Lpn;

impl InPoolWith<Ntrn> for Lpn {}
impl InPoolWith<UsdcNoble> for Lpn {}
