use serde::Deserialize;

use crate::{group::MemberOf, AnyVisitor, CurrencyDTO, Group, Matcher, MaybeAnyVisitResult};

pub type SuperGroupTestC1 = impl_::TestC1;
pub type SuperGroupTestC2 = impl_::TestC2;
pub type SuperGroupTestC3 = impl_::TestC3;
pub type SuperGroupTestC4 = impl_::TestC4;
pub type SuperGroupTestC5 = impl_::TestC5;
pub type SubGroupTestC6 = impl_::TestC6;
pub type SubGroupTestC10 = impl_::TestC10;

#[derive(Debug, Copy, Clone, Ord, PartialEq, PartialOrd, Eq, Deserialize)]
pub struct SuperGroup {}
pub type SuperGroupCurrency = CurrencyDTO<SuperGroup>;

impl Group for SuperGroup {
    const DESCR: &'static str = "super_group";

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher,
        V: AnyVisitor<Self, VisitorG = Self>,
    {
        crate::maybe_visit_any::<_, SuperGroupTestC1, _>(matcher, visitor)
            .or_else(|visitor| crate::maybe_visit_any::<_, SuperGroupTestC2, _>(matcher, visitor))
            .or_else(|visitor| crate::maybe_visit_any::<_, SuperGroupTestC3, _>(matcher, visitor))
            .or_else(|visitor| crate::maybe_visit_any::<_, SuperGroupTestC4, _>(matcher, visitor))
            .or_else(|visitor| crate::maybe_visit_any::<_, SuperGroupTestC5, _>(matcher, visitor))
            .or_else(|visitor| SubGroup::maybe_visit_member::<_, _, Self>(matcher, visitor))
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
        unreachable!()
    }

    fn maybe_visit_member<M, V, TopG>(_matcher: &M, _visitor: V) -> MaybeAnyVisitResult<TopG, V>
    where
        M: Matcher,
        V: AnyVisitor<TopG, VisitorG = TopG>,
        Self: MemberOf<TopG>,
        TopG: Group,
    {
        unreachable!()
    }
}
impl MemberOf<Self> for SuperGroup {}

#[derive(Debug, Copy, Clone, Ord, PartialEq, PartialOrd, Eq, Deserialize)]
pub struct SubGroup {}
pub type SubGroupCurrency = CurrencyDTO<SubGroup>;

impl Group for SubGroup {
    const DESCR: &'static str = "sub_group";

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher,
        V: AnyVisitor<Self, VisitorG = Self>,
    {
        Self::maybe_visit_member::<_, _, Self>(matcher, visitor)
    }

    fn maybe_visit_super_visitor<M, V, TopG>(
        matcher: &M,
        visitor: V,
    ) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher,
        V: AnyVisitor<Self, VisitorG = TopG>,
        Self: MemberOf<TopG>,
        TopG: Group,
    {
        maybe_visit::<_, Self, _>(matcher, visitor)
    }

    fn maybe_visit_member<M, V, TopG>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<TopG, V>
    where
        M: Matcher,
        V: AnyVisitor<TopG, VisitorG = TopG>,
        Self: MemberOf<TopG>,
        TopG: Group,
    {
        maybe_visit::<_, TopG, _>(matcher, visitor)
    }
}
impl MemberOf<Self> for SubGroup {}
impl MemberOf<SuperGroup> for SubGroup {}

fn maybe_visit<M, TopG, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<TopG, V>
where
    M: Matcher,
    V: AnyVisitor<TopG>,
    SubGroup: MemberOf<TopG> + MemberOf<V::VisitorG>,
    TopG: Group + MemberOf<V::VisitorG>,
{
    crate::maybe_visit_member::<_, SubGroupTestC10, TopG, _>(matcher, visitor).or_else(|visitor| {
        crate::maybe_visit_member::<_, SubGroupTestC6, TopG, _>(matcher, visitor)
    })
}

mod impl_ {
    use serde::{Deserialize, Serialize};

    use crate::{CurrencyDTO, CurrencyDef, Definition};

    use super::{SubGroup, SuperGroup};

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
    pub struct TestC1(CurrencyDTO<SuperGroup>);
    pub const TESTC1_DEFINITION: Definition =
        Definition::new("ticker#1", "ibc/bank_ticker#1", "ibc/dex_ticker#1", 6);
    pub const TESTC1: TestC1 = TestC1(CurrencyDTO::new(&TESTC1_DEFINITION));

    impl CurrencyDef for TestC1 {
        type Group = SuperGroup;

        fn definition() -> &'static Self {
            &TESTC1
        }

        fn dto(&self) -> &CurrencyDTO<Self::Group> {
            &self.0
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
    pub struct TestC2(CurrencyDTO<SuperGroup>);
    pub const TESTC2_DEFINITION: Definition =
        Definition::new("ticker#2", "ibc/bank_ticker#2", "ibc/dex_ticker#2", 6);
    pub const TESTC2: TestC2 = TestC2(CurrencyDTO::new(&TESTC2_DEFINITION));

    impl CurrencyDef for TestC2 {
        type Group = SuperGroup;

        fn definition() -> &'static Self {
            &TESTC2
        }
        fn dto(&self) -> &CurrencyDTO<Self::Group> {
            &self.0
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
    pub struct TestC3(CurrencyDTO<SuperGroup>);
    const TESTC3_DEFINITION: Definition =
        Definition::new("ticker#3", "ibc/bank_ticker#3", "ibc/dex_ticker#3", 6);
    const TESTC3: TestC3 = TestC3(CurrencyDTO::new(&TESTC3_DEFINITION));

    impl CurrencyDef for TestC3 {
        type Group = SuperGroup;

        fn definition() -> &'static Self {
            &TESTC3
        }

        fn dto(&self) -> &CurrencyDTO<Self::Group> {
            &self.0
        }
    }
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
    pub struct TestC4(CurrencyDTO<SuperGroup>);
    pub const TESTC4_DEFINITION: Definition =
        Definition::new("ticker#4", "ibc/bank_ticker#4", "ibc/dex_ticker#4", 6);
    pub const TESTC4: TestC4 = TestC4(CurrencyDTO::new(&TESTC4_DEFINITION));

    impl CurrencyDef for TestC4 {
        type Group = SuperGroup;

        fn definition() -> &'static Self {
            &TESTC4
        }

        fn dto(&self) -> &CurrencyDTO<Self::Group> {
            &self.0
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
    pub struct TestC5(CurrencyDTO<SuperGroup>);
    pub const TESTC5_DEFINITION: Definition =
        Definition::new("ticker#5", "ibc/bank_ticker#5", "ibc/dex_ticker#5", 6);
    pub const TESTC5: TestC5 = TestC5(CurrencyDTO::new(&TESTC5_DEFINITION));

    impl CurrencyDef for TestC5 {
        type Group = SuperGroup;

        fn definition() -> &'static Self {
            &TESTC5
        }

        fn dto(&self) -> &CurrencyDTO<Self::Group> {
            &self.0
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
    pub struct TestC6(CurrencyDTO<SubGroup>);
    pub const TESTC6_DEFINITION: Definition =
        Definition::new("ticker#6", "ibc/bank_ticker#6", "ibc/dex_ticker#6", 6);
    pub const TESTC6: TestC6 = TestC6(CurrencyDTO::new(&TESTC6_DEFINITION));

    impl CurrencyDef for TestC6 {
        type Group = SubGroup;

        fn definition() -> &'static Self {
            &TESTC6
        }

        fn dto(&self) -> &CurrencyDTO<Self::Group> {
            &self.0
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
    pub struct TestC10(CurrencyDTO<SubGroup>);
    pub const TESTC10_DEFINITION: Definition =
        Definition::new("ticker#10", "ibc/bank_ticker#10", "ibc/dex_ticker#10", 6);
    pub const TESTC10: TestC10 = TestC10(CurrencyDTO::new(&TESTC10_DEFINITION));

    impl CurrencyDef for TestC10 {
        type Group = SubGroup;

        fn definition() -> &'static Self {
            &TESTC10
        }

        fn dto(&self) -> &CurrencyDTO<Self::Group> {
            &self.0
        }
    }
}
