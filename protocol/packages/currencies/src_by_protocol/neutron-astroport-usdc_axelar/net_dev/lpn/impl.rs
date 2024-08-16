use sdk::schemars;

use crate::{define_currency, Lpns};

define_currency!(
    UsdcAxelar,
    "USDC_AXELAR",
    "ibc/88012ABE034CE754022417BFEDF29F8B16C5B3338386EA20298ADCECA8329019", // transfer/channel-1/transfer/channel-8/uausdc
    "ibc/F91EA2C0A23697A1048E08C2F787E3A58AC6F706A1CD2257A504925158CFC0F3", // transfer/channel-8/uausdc
    Lpns,
    6
);

pub use UsdcAxelar as Lpn;
