use sdk::schemars;

use crate::{define_currency, define_symbol};

define_symbol! {
    USDC_NOBLE {
        // full ibc route: transfer/channel-0/transfer/channel-750/uusdc
        bank: "ibc/F5FABF52B54E65064B57BF6DBD8E5FAD22CEE9F4B8A57ADBB20CCD0173AA72A4",
        // full ibc route: transfer/channel-750/uusdc
        dex: "ibc/498A0751C798A0D9A389AA3691123DADA57DAA4FE165D5C75894505B876BA6E4",
    }
}
define_currency!(UsdcNoble, USDC_NOBLE, 6);
