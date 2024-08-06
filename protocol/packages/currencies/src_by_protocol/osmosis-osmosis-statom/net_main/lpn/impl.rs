use sdk::schemars;

use crate::{define_currency, define_symbol, Lpns};

define_symbol! {
    ST_ATOM {
        // full ibc route: transfer/channel-0/transfer/channel-326/stuatom
        bank: "ibc/FCFF8B19C61677F3B78E2A5AE3B4A34A8D23858D16905F253B8438B3AFD07FF8",
        // full ibc route: transfer/channel-326/stuatom
        dex: "ibc/C140AFD542AE77BD7DCC83F13FDD8C5E5BB8C4929785E6EC2F4C636F98F17901",
    }
}
define_currency!(StAtom, ST_ATOM, Lpns, 6);

pub use StAtom as Lpn;
