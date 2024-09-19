use currency::InPoolWith;
use sdk::schemars;

use crate::{define_currency, LeaseC2, LeaseC7, Lpns, Nls};

define_currency!(Lpn, "LPN", "ibc/test_LPN", "ibc/test_DEX_LPN", Lpns, 6);

impl InPoolWith<LeaseC2> for Lpn {}
impl InPoolWith<Nls> for Lpn {}
impl InPoolWith<LeaseC7> for Lpn {}
