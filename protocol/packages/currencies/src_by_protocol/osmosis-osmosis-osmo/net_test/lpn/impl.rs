use currency::InPoolWith;
use sdk::schemars;

use crate::{define_currency, lease::impl_mod::UsdcAxelar, payment::only::impl_mod::Atom, Lpns};

define_currency!(
    Osmo,
    "OSMO",
    "ibc/ED07A3391A112B175915CD8FAF43A2DA8E4790EDE12566649D0C2F97716B8518", // transfer/channel-0/uosmo
    "uosmo",
    Lpns,
    6
);

pub use Osmo as Lpn;

impl InPoolWith<UsdcAxelar> for Lpn {}
impl InPoolWith<Atom> for Lpn {}
