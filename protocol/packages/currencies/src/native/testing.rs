use serde::{Deserialize, Serialize};

use currency::{
    CurrencyDTO, CurrencyDef, Definition, InPoolWith, Matcher, MaybePairsVisitorResult, PairsGroup,
    PairsVisitor,
};
use sdk::schemars::JsonSchema;

use crate::{lease::LeaseC5, lpn::Lpn, payment};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
#[schemars(crate = "sdk::schemars")]
pub struct Nls(CurrencyDTO<super::Group>);

impl CurrencyDef for Nls {
    type Group = super::Group;

    #[inline]
    fn definition() -> &'static Self {
        const {
            &Nls(CurrencyDTO::new(
                const { &Definition::new("NLS", "unls", "ibc/dex_NLS", 6) },
            ))
        }
    }

    #[inline]
    fn dto(&self) -> &CurrencyDTO<Self::Group> {
        &self.0
    }
}

impl PairsGroup for Nls {
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

impl InPoolWith<LeaseC5> for Nls {}
