use sdk::schemars;

use crate::{define_currency, define_symbol, Lpns};

define_symbol! {
    USDC {
        // full ibc route: transfer/channel-0/transfer/channel-3/uausdc
        bank: "ibc/5DE4FCAF68AE40F81F738C857C0D95F7C1BC47B00FA1026E85C1DD92524D4A11",
        // full ibc route: transfer/channel-3/uausdc
        dex: "ibc/6F34E1BD664C36CE49ACC28E60D62559A5F96C4F9A6CCE4FC5A67B2852E24CFE",
    }
}
define_currency!(Usdc, USDC, Lpns, 6);

pub use Usdc as Lpn;
