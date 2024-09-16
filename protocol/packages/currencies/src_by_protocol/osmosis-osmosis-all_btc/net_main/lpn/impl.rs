use sdk::schemars;

use currency::InPoolWith;

use crate::{define_currency, lease::impl_mod::UsdcNoble, payment::only::impl_mod::AllSol, Lpns};

define_currency!(
    AllBtc,
    "ALL_BTC",
    "ibc/E45CFCB959F4F6D1065B7033EE49A88E606E6AD82E75725219B3D68B0FA89987", // transfer/channel-0/allBTC
    "factory/osmo1z6r6qdknhgsc0zeracktgpcxf43j6sekq07nw8sxduc9lg0qjjlqfu25e3/alloyed/allBTC",
    Lpns,
    8
);
pub use AllBtc as Lpn;

impl InPoolWith<AllSol> for Lpn {}
impl InPoolWith<UsdcNoble> for Lpn {}
