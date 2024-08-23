use sdk::schemars;

use crate::{define_currency, Lpns};

define_currency!(
    UsdcAxelar,
    "USDC_AXELAR",
    "ibc/AB087F13998D7C443EC07D6CDE6E04FDF203BF4324BB6953DF2E06439A52FD02", // transfer/channel-3/transfer/channel-8/uausdc
    "ibc/F91EA2C0A23697A1048E08C2F787E3A58AC6F706A1CD2257A504925158CFC0F3", // transfer/channel-8/uausdc
    Lpns,
    6
);

pub use UsdcAxelar as Lpn;
