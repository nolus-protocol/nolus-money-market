use sdk::schemars;

use currency::InPoolWith;

use crate::{define_currency, lease::impl_mod::Osmo, Lpns};

define_currency!(
    Usdc,
    "USDC",
    "ibc/5DE4FCAF68AE40F81F738C857C0D95F7C1BC47B00FA1026E85C1DD92524D4A11", // transfer/channel-0/transfer/channel-3/uausdc
    "ibc/6F34E1BD664C36CE49ACC28E60D62559A5F96C4F9A6CCE4FC5A67B2852E24CFE", // transfer/channel-3/uausdc
    Lpns,
    6
);

pub use Usdc as Lpn;

impl InPoolWith<Osmo> for Lpn {}
