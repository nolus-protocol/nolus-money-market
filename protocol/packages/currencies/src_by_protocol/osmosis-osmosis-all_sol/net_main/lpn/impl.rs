use sdk::schemars;

use currency::InPoolWith;

use crate::{define_currency, payment::only::impl_mod::AllBtc, Lpns};

define_currency!(
    AllSol,
    "ALL_SOL",
    "ibc/762E1E45658845A12E214A91C3C05FDFC5951D60404FAADA225A369A96DCD9A9", // transfer/channel-0/allSOL
    "factory/osmo1n3n75av8awcnw4jl62n3l48e6e4sxqmaf97w5ua6ddu4s475q5qq9udvx4/alloyed/allSOL",
    Lpns,
    9
);
pub use AllSol as Lpn;

impl InPoolWith<AllBtc> for Lpn {}
