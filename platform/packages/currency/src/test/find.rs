use std::{fmt::Debug, marker::PhantomData};

use crate::{CurrencyDTO, CurrencyDef, Group, GroupFindMapT, Matcher, MemberOf, PairsGroup};

pub struct FindCurrencyBySymbol<Matcher, TargetG>(Matcher, PhantomData<TargetG>);

impl<Matcher, TargetG> FindCurrencyBySymbol<Matcher, TargetG> {
    pub fn with_matcher(m: Matcher) -> Self {
        Self(m, PhantomData)
    }
}

impl<Matcher, TargetG> Debug for FindCurrencyBySymbol<Matcher, TargetG> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("FindCurrencyBySymbol")
            .field(&"matcher")
            .finish()
    }
}

impl<MatcherImpl, TargetG> GroupFindMapT for FindCurrencyBySymbol<MatcherImpl, TargetG>
where
    MatcherImpl: Matcher,
    TargetG: Group,
{
    type TargetG = TargetG;

    type Outcome = CurrencyDTO<TargetG>;

    fn on<C>(self, def: &CurrencyDTO<C::Group>) -> Result<Self::Outcome, Self>
    where
        C: CurrencyDef + PairsGroup<CommonGroup = TargetG::TopG>,
        C::Group: MemberOf<TargetG> + MemberOf<TargetG::TopG>,
    {
        if self.0.r#match(def.definition()) {
            Ok(def.into_super_group())
        } else {
            Err(self)
        }
    }
}
