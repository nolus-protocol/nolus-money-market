use sdk::schemars;

use currency::{Matcher, MaybePairsVisitorResult, PairsVisitor};

use crate::{define_currency, Lpns};

define_currency!(
    UsdcNoble,
    "USDC_NOBLE",
    "ibc/F5FABF52B54E65064B57BF6DBD8E5FAD22CEE9F4B8A57ADBB20CCD0173AA72A4", // transfer/channel-0/transfer/channel-750/uusdc
    "ibc/498A0751C798A0D9A389AA3691123DADA57DAA4FE165D5C75894505B876BA6E4", // transfer/channel-750/uusdc
    Lpns,
    6
);

pub use UsdcNoble as Lpn;

impl PairsGroup for Lpn {
    type CommonGroup = PaymentGroup;

    fn maybe_visit<M, V>(_matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<VisitedG = Self::CommonGroup>,
    {
        currency::visit_noone(visitor) // TODO
    }
}
