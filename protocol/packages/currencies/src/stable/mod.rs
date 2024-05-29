/// A currency that is bound to a stable fiat, for example USDC or USDT.
/// It should be a member of the [crate::PaymentGroup].
// Explicitly list all long protocols to avoid potential errors when adding short protocols.
#[cfg(any(
    feature = "osmosis-osmosis-usdc_axelar",
    feature = "osmosis-osmosis-usdc_noble",
    feature = "neutron-astroport-usdc_axelar",
    feature = "neutron-astroport-usdc_noble"
))]
pub use crate::Lpn as Stable;
