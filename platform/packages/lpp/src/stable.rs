use serde::{Deserialize, Serialize};

use currency::{
    AnyVisitor, CurrencyDTO, CurrencyDef, Definition, Group, Matcher, MaybeAnyVisitResult, MemberOf,
};
use finance::coin::Coin;
use sdk::schemars::{self, JsonSchema};

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize, JsonSchema,
)]
pub struct Stable(CurrencyDTO<StableCurrencyGroup>);
const STABLE_DEFINITION: Definition = Definition::new("STABLE", "N/A_N/A_N/A", "N/A_N/A_N/A", 0);
const STABLE: Stable = Stable(CurrencyDTO::new(&STABLE_DEFINITION));

impl CurrencyDef for Stable {
    type Group = StableCurrencyGroup;

    fn definition() -> &'static Self {
        &STABLE
    }

    fn dto(&self) -> &CurrencyDTO<Self::Group> {
        &self.0
    }
}

pub type CoinStable = Coin<Stable>;

#[derive(Debug, Copy, Clone, Ord, PartialEq, PartialOrd, Eq, Deserialize)]
pub struct StableCurrencyGroup;
impl Group for StableCurrencyGroup {
    const DESCR: &'static str = "stable currency";

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher,
        V: AnyVisitor<Self, VisitorG = Self>,
    {
        Self::maybe_visit_member(matcher, visitor)
    }

    fn maybe_visit_super_visitor<M, V, TopG>(
        _matcher: &M,
        _visitor: V,
    ) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher,
        V: AnyVisitor<Self, VisitorG = TopG>,
        Self: MemberOf<TopG>,
        TopG: Group,
    {
        unreachable!("There is no parent group to this one!")
    }

    fn maybe_visit_member<M, V, TopG>(_matcher: &M, visitor: V) -> MaybeAnyVisitResult<TopG, V>
    where
        M: Matcher,
        V: AnyVisitor<TopG, VisitorG = TopG>,
        Self: MemberOf<TopG>,
        TopG: Group,
    {
        MaybeAnyVisitResult::Ok(visitor.on::<Stable>(&STABLE)) // we accept ANY currency to allow any stable@protocol to be a member
    }
}

impl MemberOf<Self> for StableCurrencyGroup {}
