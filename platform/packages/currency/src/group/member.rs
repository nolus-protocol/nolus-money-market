use crate::{CurrencyDef, Group};

/// Member type of a group
///
/// Express a 'member-of' relation of currency *types* in compile-time.
pub trait MemberOf<G>
where
    G: Group,
{
}

impl<G, C> MemberOf<G> for C
where
    C: CurrencyDef,
    C::Group: MemberOf<G>,
    G: Group,
{
}
