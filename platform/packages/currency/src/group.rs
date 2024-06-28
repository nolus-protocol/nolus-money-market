use std::fmt::Debug;

use crate::{AnyVisitor, Currency, Matcher, MaybeAnyVisitResult};

// TODO rename to GroupMember???
// TODO try to remove the + MemberOf<Self>
pub trait Group: Copy + Clone + Debug + PartialEq + MemberOf<Self> {
    const DESCR: &'static str;

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher + ?Sized,
        V: AnyVisitor<VisitedG = Self>;

    fn maybe_visit_member<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher + ?Sized,
        V: AnyVisitor,
        Self: MemberOf<V::VisitedG>;
}

pub trait MemberOf<G> {}

impl<G, C> MemberOf<G> for C
where
    C: Currency,
    C::Group: MemberOf<G>,
{
}

#[cfg(test)]
mod test {
    use std::any::TypeId;

    use crate::{
        test::{Expect, SubGroupTestC1, SuperGroup, SuperGroupTestC1},
        Group, TypeMatcher,
    };

    #[test]
    fn visit_any_same_group() {
        let visitor = Expect::<SuperGroupTestC1, SuperGroup>::default();
        let matcher = TypeMatcher::new(TypeId::of::<SuperGroupTestC1>());
        assert_eq!(Ok(Ok(true)), SuperGroup::maybe_visit(&matcher, visitor));
    }

    #[test]
    fn visit_any_sub_group() {
        let visitor = Expect::<SubGroupTestC1, SuperGroup>::default();
        let matcher = TypeMatcher::new(TypeId::of::<SubGroupTestC1>());
        assert_eq!(Ok(Ok(true)), SuperGroup::maybe_visit(&matcher, visitor));
    }
    //     let v_nls = Expect::<SuperGroupTestC2, SuperGroup>::default();
    //     assert_eq!(
    //         Ok(true),
    //         Tickers::visit_any::<SuperGroup, _>(SuperGroupTestC2::TICKER, v_nls)
    //     );

    //     assert_eq!(
    //         Err(Error::not_in_currency_group::<_, Tickers, SuperGroup>(
    //             SubGroupTestC1::BANK_SYMBOL
    //         )),
    //         Tickers::visit_any::<SuperGroup, _>(
    //             SubGroupTestC1::BANK_SYMBOL,
    //             ExpectUnknownCurrency::<SuperGroup>::default()
    //         )
    //     );
}
