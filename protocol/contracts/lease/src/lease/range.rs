use crate::finance::Price;

/// A range of prices for which a lease is in steady position
///
/// A position is steady when no liquidations, automatic close or warnings are fired.
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub(super) struct SteadyPriceRange<Asset>
where
    Asset: 'static,
{
    above_excl: Price<Asset>,
    below_incl: Option<Price<Asset>>,
}

impl<Asset> SteadyPriceRange<Asset>
where
    Asset: 'static,
{
    pub fn new(above_excl: Price<Asset>, below_incl: Option<Price<Asset>>) -> Self {
        Self {
            above_excl,
            below_incl,
        }
        // TODO debug_assert!(self.invariant())
    }

    pub fn above_excl(&self) -> Price<Asset> {
        self.above_excl
    }

    pub fn may_below_incl(&self) -> Option<Price<Asset>> {
        self.below_incl
    }
}
