/// A currency that is bound to a stable fiat, for example USDC or USDT.
/// It should be a member of the [crate::PaymentGroup].
// Explicitly list all long protocols to avoid potential errors when adding short protocols.
pub use crate::Lpn as Stable;
