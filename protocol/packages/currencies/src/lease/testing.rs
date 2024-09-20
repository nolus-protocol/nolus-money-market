use serde::{Deserialize, Serialize};

use currency::{
    AnyVisitor, CurrencyDTO, CurrencyDef, Definition, Group, InPoolWith, Matcher,
    MaybeAnyVisitResult, MaybePairsVisitorResult, MemberOf, PairsGroup, PairsVisitor,
};
use sdk::schemars::JsonSchema;

use crate::{lpn::impl_mod::Lpn, native::impl_mod::Nls, payment};

pub(super) fn maybe_visit<M, V, VisitedG>(
    matcher: &M,
    visitor: V,
) -> MaybeAnyVisitResult<VisitedG, V>
where
    super::Group: MemberOf<VisitedG>,
    M: Matcher,
    V: AnyVisitor<VisitedG>,
    VisitedG: Group<TopG = payment::Group>,
{
    use currency::maybe_visit_member as visit;

    visit::<_, LeaseC1, VisitedG, _>(matcher, visitor)
        .or_else(|visitor| visit::<_, LeaseC2, VisitedG, _>(matcher, visitor))
        .or_else(|visitor| visit::<_, LeaseC3, VisitedG, _>(matcher, visitor))
        .or_else(|visitor| visit::<_, LeaseC4, VisitedG, _>(matcher, visitor))
        .or_else(|visitor| visit::<_, LeaseC5, VisitedG, _>(matcher, visitor))
        .or_else(|visitor| visit::<_, LeaseC6, VisitedG, _>(matcher, visitor))
        .or_else(|visitor| visit::<_, LeaseC7, VisitedG, _>(matcher, visitor))
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
#[schemars(crate = "sdk::schemars")]
pub struct LeaseC1(CurrencyDTO<super::Group>);

impl CurrencyDef for LeaseC1 {
    type Group = super::Group;

    #[inline]
    fn definition() -> &'static Self {
        const {
            &Self(CurrencyDTO::new(
                const { &Definition::new("LC1", "ibc/bank_LC1", "ibc/dex_LC1", 6) },
            ))
        }
    }

    #[inline]
    fn dto(&self) -> &CurrencyDTO<Self::Group> {
        &self.0
    }
}

impl PairsGroup for LeaseC1 {
    type CommonGroup = payment::Group;

    #[inline]
    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        use currency::maybe_visit_buddy as visit;

        visit::<LeaseC2, _, _>(matcher, visitor)
            .or_else(|visitor| visit::<LeaseC3, _, _>(matcher, visitor))
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
#[schemars(crate = "sdk::schemars")]
pub struct LeaseC2(CurrencyDTO<super::Group>);

impl CurrencyDef for LeaseC2 {
    type Group = super::Group;

    #[inline]
    fn definition() -> &'static Self {
        const {
            &Self(CurrencyDTO::new(
                const { &Definition::new("LC2", "ibc/bank_LC2", "ibc/dex_LC2", 6) },
            ))
        }
    }

    #[inline]
    fn dto(&self) -> &CurrencyDTO<Self::Group> {
        &self.0
    }
}

impl PairsGroup for LeaseC2 {
    type CommonGroup = payment::Group;

    #[inline]
    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        use currency::maybe_visit_buddy as visit;

        visit::<Lpn, _, _>(matcher, visitor)
    }
}

impl InPoolWith<LeaseC1> for LeaseC2 {}

impl InPoolWith<LeaseC3> for LeaseC2 {}

impl InPoolWith<LeaseC4> for LeaseC2 {}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
#[schemars(crate = "sdk::schemars")]
pub struct LeaseC3(CurrencyDTO<super::Group>);

impl CurrencyDef for LeaseC3 {
    type Group = super::Group;

    #[inline]
    fn definition() -> &'static Self {
        const {
            &Self(CurrencyDTO::new(
                const { &Definition::new("LC3", "ibc/bank_LC3", "ibc/dex_LC3", 6) },
            ))
        }
    }

    #[inline]
    fn dto(&self) -> &CurrencyDTO<Self::Group> {
        &self.0
    }
}

impl PairsGroup for LeaseC3 {
    type CommonGroup = payment::Group;

    #[inline]
    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        use currency::maybe_visit_buddy as visit;

        visit::<LeaseC2, _, _>(matcher, visitor)
    }
}

impl InPoolWith<LeaseC1> for LeaseC3 {}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
#[schemars(crate = "sdk::schemars")]
pub struct LeaseC4(CurrencyDTO<super::Group>);

impl CurrencyDef for LeaseC4 {
    type Group = super::Group;

    #[inline]
    fn definition() -> &'static Self {
        const {
            &Self(CurrencyDTO::new(
                const { &Definition::new("LC4", "ibc/bank_LC4", "ibc/dex_LC4", 18) },
            ))
        }
    }

    #[inline]
    fn dto(&self) -> &CurrencyDTO<Self::Group> {
        &self.0
    }
}

impl PairsGroup for LeaseC4 {
    type CommonGroup = payment::Group;

    #[inline]
    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        use currency::maybe_visit_buddy as visit;

        visit::<LeaseC2, _, _>(matcher, visitor)
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
#[schemars(crate = "sdk::schemars")]
pub struct LeaseC5(CurrencyDTO<super::Group>);

impl CurrencyDef for LeaseC5 {
    type Group = super::Group;

    #[inline]
    fn definition() -> &'static Self {
        const {
            &Self(CurrencyDTO::new(
                const { &Definition::new("LC5", "ibc/bank_LC5", "ibc/dex_LC5", 6) },
            ))
        }
    }

    #[inline]
    fn dto(&self) -> &CurrencyDTO<Self::Group> {
        &self.0
    }
}

impl PairsGroup for LeaseC5 {
    type CommonGroup = payment::Group;

    #[inline]
    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        use currency::maybe_visit_buddy as visit;

        visit::<Nls, _, _>(matcher, visitor)
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
#[schemars(crate = "sdk::schemars")]
pub struct LeaseC6(CurrencyDTO<super::Group>);

impl CurrencyDef for LeaseC6 {
    type Group = super::Group;

    #[inline]
    fn definition() -> &'static Self {
        const {
            &Self(CurrencyDTO::new(
                const { &Definition::new("LC6", "ibc/bank_LC6", "ibc/dex_LC6", 8) },
            ))
        }
    }

    #[inline]
    fn dto(&self) -> &CurrencyDTO<Self::Group> {
        &self.0
    }
}

impl PairsGroup for LeaseC6 {
    type CommonGroup = payment::Group;

    #[inline]
    fn maybe_visit<M, V>(_: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        // let's stay detached from the swap tree for some corner cases.
        currency::visit_noone(visitor)
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
#[schemars(crate = "sdk::schemars")]
pub struct LeaseC7(CurrencyDTO<super::Group>);

impl CurrencyDef for LeaseC7 {
    type Group = super::Group;

    #[inline]
    fn definition() -> &'static Self {
        const {
            &Self(CurrencyDTO::new(
                const { &Definition::new("LC7", "ibc/bank_LC7", "ibc/dex_LC7", 4) },
            ))
        }
    }

    #[inline]
    fn dto(&self) -> &CurrencyDTO<Self::Group> {
        &self.0
    }
}

impl PairsGroup for LeaseC7 {
    type CommonGroup = payment::Group;

    #[inline]
    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        use currency::maybe_visit_buddy as visit;

        visit::<Lpn, _, _>(matcher, visitor)
    }
}

#[cfg(test)]
mod test {
    use currency::CurrencyDef as _;

    use crate::{
        lpn::{impl_mod::Lpn, Group as Lpns},
        native::impl_mod::Nls,
        test_impl::{
            maybe_visit_on_bank_symbol_err, maybe_visit_on_bank_symbol_impl,
            maybe_visit_on_ticker_err, maybe_visit_on_ticker_impl,
        },
    };

    use super::{
        super::Group as LeaseGroup, LeaseC1, LeaseC2, LeaseC3, LeaseC4, LeaseC5, LeaseC6, LeaseC7,
    };

    #[test]
    fn maybe_visit_on_ticker() {
        maybe_visit_on_ticker_impl::<LeaseC1, LeaseGroup>();
        maybe_visit_on_ticker_impl::<LeaseC2, LeaseGroup>();
        maybe_visit_on_ticker_impl::<LeaseC3, LeaseGroup>();
        maybe_visit_on_ticker_impl::<LeaseC4, LeaseGroup>();
        maybe_visit_on_ticker_impl::<LeaseC5, LeaseGroup>();
        maybe_visit_on_ticker_impl::<LeaseC6, LeaseGroup>();
        maybe_visit_on_ticker_impl::<LeaseC7, LeaseGroup>();
        maybe_visit_on_ticker_err::<Lpn, Lpns>(Lpn::bank());
        maybe_visit_on_ticker_err::<LeaseC2, LeaseGroup>(LeaseC2::bank());
        maybe_visit_on_ticker_err::<LeaseC3, LeaseGroup>(LeaseC3::dex());
    }

    #[test]
    fn maybe_visit_on_bank_symbol() {
        maybe_visit_on_bank_symbol_impl::<LeaseC1, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<LeaseC2, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<LeaseC3, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<LeaseC4, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<LeaseC5, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<LeaseC6, LeaseGroup>();
        maybe_visit_on_bank_symbol_impl::<LeaseC7, LeaseGroup>();
        maybe_visit_on_bank_symbol_err::<Lpn, Lpns>(Lpn::ticker());
        maybe_visit_on_bank_symbol_err::<LeaseC1, LeaseGroup>(LeaseC1::ticker());
        maybe_visit_on_bank_symbol_err::<LeaseC1, LeaseGroup>(LeaseC1::dex());
        maybe_visit_on_bank_symbol_err::<LeaseC1, LeaseGroup>(Lpn::ticker());
        maybe_visit_on_bank_symbol_err::<LeaseC1, LeaseGroup>(Nls::bank());
        maybe_visit_on_bank_symbol_err::<LeaseC1, LeaseGroup>(Nls::ticker());
        maybe_visit_on_bank_symbol_err::<LeaseC5, LeaseGroup>(LeaseC5::ticker());
    }
}
