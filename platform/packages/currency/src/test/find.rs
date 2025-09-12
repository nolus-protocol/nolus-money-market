use std::fmt::Debug;

use crate::{CurrencyDTO, CurrencyDef, FindMapT, Group, Matcher, MemberOf, PairsGroup};

pub struct FindCurrencyBySymbol<Matcher>(Matcher);

impl<Matcher> FindCurrencyBySymbol<Matcher> {
    pub fn with_matcher(m: Matcher) -> Self {
        Self(m)
    }
}

impl<Matcher> Debug for FindCurrencyBySymbol<Matcher> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("FindCurrencyBySymbol")
            .field(&"matcher")
            .finish()
    }
}

impl<MatcherImpl, VisitedG> FindMapT<VisitedG> for FindCurrencyBySymbol<MatcherImpl>
where
    MatcherImpl: Matcher,
    VisitedG: Group,
{
    type Outcome = CurrencyDTO<VisitedG>;

    fn on<C>(self, def: &CurrencyDTO<C::Group>) -> Result<Self::Outcome, Self>
    where
        C: CurrencyDef + PairsGroup<CommonGroup = VisitedG::TopG>,
        C::Group: MemberOf<VisitedG> + MemberOf<VisitedG::TopG>,
    {
        if self.0.r#match(def.definition()) {
            Ok(def.into_super_group())
        } else {
            Err(self)
        }
    }
}
