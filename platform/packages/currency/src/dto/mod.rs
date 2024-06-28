use std::{
    any::TypeId,
    fmt::{Debug, Display, Formatter},
    marker::PhantomData,
};

use sdk::schemars::{gen::SchemaGenerator, schema::Schema, JsonSchema};
use serde::{Deserialize, Serialize};

use crate::{group::MemberOf, Currency, Group, TypeMatcher};

use super::{AnyVisitor, AnyVisitorResult};

mod unchecked;

pub type MaybeAnyVisitResult<V> = Result<AnyVisitorResult<V>, V>;

/// Data-Transferable currency belonging to a group
///
/// This is a value type designed for efficient representation, data transfer and storage.
/// `GroupMember` specifies which currencies are valid instances of this type.
#[derive(Copy, Clone, Debug, Eq, Serialize, Deserialize)]
#[serde(try_from = "unchecked::CurrencyDTO", into = "unchecked::CurrencyDTO")]
pub struct CurrencyDTO<G>
where
    G: Group,
{
    id: TypeId,
    _group_member: PhantomData<G>,
}

impl<G> CurrencyDTO<G>
where
    G: Group,
{
    pub fn from_currency_type<C>() -> Self
    where
        C: Currency + MemberOf<G>,
    {
        let _res = TypeId::of::<C>().into();
        // TODO debug_assert!(_res.into_currency_type(Expect))
        _res
    }

    pub fn into_currency_type<V>(self, visitor: V) -> AnyVisitorResult<V>
    where
        V: AnyVisitor<VisitedG = G>,
        G: MemberOf<V::VisitedG>,
    {
        G::maybe_visit(&TypeMatcher::new(self), visitor).unwrap_or_else(|_| {
            panic!(
                r#"Found an invalid currency instance! "{:?}" did not match "{}" !"#,
                self,
                G::DESCR
            )
        })
    }
}

impl<G> Display for CurrencyDTO<G>
where
    G: Group,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", unchecked::CurrencyDTO::from(*self)))
    }
}

impl<G> From<CurrencyDTO<G>> for TypeId
where
    G: Group,
{
    fn from(value: CurrencyDTO<G>) -> Self {
        value.id
    }
}

impl<G> From<TypeId> for CurrencyDTO<G>
where
    G: Group,
{
    fn from(type_id: TypeId) -> Self {
        Self {
            id: type_id,
            _group_member: PhantomData,
        }
    }
}

impl<G1, G2> PartialEq<CurrencyDTO<G1>> for CurrencyDTO<G2>
where
    G1: Group,
    G2: Group,
{
    fn eq(&self, other: &CurrencyDTO<G1>) -> bool {
        TypeId::from(*self).eq(&TypeId::from(*other))
    }
}

impl<G> JsonSchema for CurrencyDTO<G>
where
    G: Group,
{
    fn schema_name() -> String {
        unchecked::CurrencyDTO::schema_name()
    }

    fn json_schema(gen: &mut SchemaGenerator) -> Schema {
        unchecked::CurrencyDTO::json_schema(gen)
    }
}

#[cfg(test)]
mod test {
    use std::any::TypeId;

    use crate::{
        test::{SubGroup, SubGroupCurrency, SuperGroup, SuperGroupTestC1, SuperGroupTestC2},
        CurrencyDTO,
    };

    #[test]
    fn eq_same_type() {
        assert_eq!(
            CurrencyDTO::<SuperGroup>::from(TypeId::of::<SuperGroupTestC1>()),
            CurrencyDTO::<SuperGroup>::from(TypeId::of::<SuperGroupTestC1>())
        );

        assert_ne!(
            CurrencyDTO::<SuperGroup>::from(TypeId::of::<SuperGroupTestC1>()),
            CurrencyDTO::<SuperGroup>::from(TypeId::of::<SuperGroupTestC2>())
        );
    }

    #[test]
    fn eq_other_type() {
        assert_ne!(
            CurrencyDTO::<SuperGroup>::from(TypeId::of::<SuperGroupTestC1>()),
            CurrencyDTO::<SubGroup>::from(TypeId::of::<SubGroupCurrency>())
        );
    }
}
