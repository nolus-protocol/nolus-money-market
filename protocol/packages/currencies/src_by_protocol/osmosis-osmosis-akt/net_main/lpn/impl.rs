use sdk::schemars;

use currency::InPoolWith;

use crate::{define_currency, payment::only::impl_mod::Osmo, Lpns};

define_currency!(
    Akt,
    "AKT",
    "ibc/ADC63C00000CA75F909D2BE3ACB5A9980BED3A73B92746E0FCE6C67414055459", // transfer/channel-0/transfer/channel-1/uakt
    "ibc/1480B8FD20AD5FCAE81EA87584D269547DD4D436843C1D20F15E00EB64743EF4", // transfer/channel-1/uakt
    Lpns,
    6
);
pub use Akt as Lpn;

impl InPoolWith<Osmo> for Lpn {}
