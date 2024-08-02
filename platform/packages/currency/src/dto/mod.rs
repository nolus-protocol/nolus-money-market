use std::{
    any::TypeId,
    fmt::{Debug, Display, Formatter},
    marker::PhantomData,
};

use sdk::schemars::{gen::SchemaGenerator, schema::Schema, JsonSchema};
use serde::{Deserialize, Serialize};

#[cfg(any(test, feature = "testing"))]
use crate::SymbolSlice;
use crate::{
    error::{Error, Result},
    group::MemberOf,
    never::{self, Never},
    Currency, Definition, Group, GroupVisit as _, MaybeAnyVisitResult, Symbol, SymbolStatic,
    Tickers, TypeMatcher,
};

use super::{AnyVisitor, AnyVisitorResult};

mod unchecked;

/// Data-Transferable currency belonging to a group
///
/// This is a value type designed for efficient representation, data transfer and storage.
/// `GroupMember` specifies which currencies are valid instances of this type.
#[derive(Copy, Clone, Debug, Eq, Ord, PartialOrd, Serialize, Deserialize)]
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
        let id = TypeId::of::<C>();
        Self {
            id,
            _group_member: PhantomData,
        }
    }

    pub fn may_into_currency_type<SubG, V>(self, visitor: V) -> MaybeAnyVisitResult<SubG, V>
    where
        SubG: Group + MemberOf<G>,
        V: AnyVisitor<SubG, VisitorG = G>,
    {
        SubG::maybe_visit_super_visitor(&TypeMatcher::new(self.id), visitor)
    }

    pub fn into_currency_super_group_type<TopG, V>(self, visitor: V) -> AnyVisitorResult<G, V>
    where
        TopG: Group,
        G: MemberOf<TopG>,
        V: AnyVisitor<G, VisitorG = TopG>,
    {
        G::maybe_visit_super_visitor(&TypeMatcher::new(self.id), visitor)
            .unwrap_or_else(|_| self.unexpected::<V>())
    }

    pub fn into_currency_type<V>(self, visitor: V) -> AnyVisitorResult<G, V>
    where
        V: AnyVisitor<G, VisitorG = G>,
    {
        G::maybe_visit(&TypeMatcher::new(self.id), visitor)
            .unwrap_or_else(|_| self.unexpected::<V>())
    }

    pub fn into_super_group<SuperG>(self) -> CurrencyDTO<SuperG>
    where
        SuperG: Group,
        G: MemberOf<SuperG>,
    {
        CurrencyDTO::<SuperG> {
            id: self.id,
            _group_member: PhantomData,
        }
    }

    pub fn into_symbol<S>(self) -> SymbolStatic
    where
        S: Symbol,
    {
        struct SymbolRetriever<G, S> {
            visited_g: PhantomData<G>,
            symbol: PhantomData<S>,
        }

        impl<G, S> AnyVisitor<G> for SymbolRetriever<G, S>
        where
            G: Group,
            S: Symbol,
        {
            type VisitorG = G;

            type Output = SymbolStatic;

            type Error = Never;

            fn on<C>(self) -> AnyVisitorResult<G, Self>
            where
                C: Definition,
            {
                Ok(S::symbol::<C>())
            }
        }

        never::safe_unwrap(self.into_currency_type(SymbolRetriever {
            visited_g: PhantomData,
            symbol: PhantomData::<S>,
        }))
    }

    pub fn of_currency<C>(&self) -> Result<()>
    where
        C: Currency + MemberOf<G>,
    {
        if self == &dto::<C, G>() {
            Ok(())
        } else {
            Err(Error::currency_mismatch::<C, _>(self))
        }
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn from_symbol<S>(symbol: &SymbolSlice) -> Result<CurrencyDTO<G>>
    where
        S: Symbol<Group = G>,
    {
        struct TypeToCurrency<G>(PhantomData<G>);
        impl<G> AnyVisitor<G> for TypeToCurrency<G>
        where
            G: Group,
        {
            type VisitorG = G;
            type Output = CurrencyDTO<G>;

            type Error = Error;

            fn on<C>(self) -> AnyVisitorResult<G, Self>
            where
                C: Currency + MemberOf<G>,
            {
                Ok(dto::<C, G>())
            }
        }
        S::visit_any(symbol, TypeToCurrency(PhantomData))
    }

    fn unexpected<V>(self) -> AnyVisitorResult<G, V>
    where
        V: AnyVisitor<G>,
        G: MemberOf<V::VisitorG>,
    {
        panic!(
            r#"Found an invalid currency instance! "{:?}" did not match "{}" !"#,
            self,
            G::DESCR
        )
    }
}

impl<G, RhsG> PartialEq<CurrencyDTO<RhsG>> for CurrencyDTO<G>
where
    G: Group,
    RhsG: Group,
{
    fn eq(&self, other: &CurrencyDTO<RhsG>) -> bool {
        self.id.eq(&other.id)
    }
}

pub fn dto<C, G>() -> CurrencyDTO<G>
where
    C: Currency + MemberOf<G>,
    G: Group,
{
    CurrencyDTO::from_currency_type::<C>()
}

pub fn symbol<C, S>() -> SymbolStatic
where
    C: Currency,
    S: Symbol,
{
    dto::<C, C::Group>().into_symbol::<S>()
}

/// Prepare a human-friendly representation of a currency
pub fn to_string<C>() -> SymbolStatic
where
    C: Currency,
{
    symbol::<C, Tickers<C::Group>>()
}

impl<G> Display for CurrencyDTO<G>
where
    G: Group,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        unchecked::CurrencyDTO::from(*self).fmt(f)
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

    use crate::{
        test::{self, SubGroup, SubGroupTestC1, SuperGroup, SuperGroupTestC1, SuperGroupTestC2},
        BankSymbols, Currency, CurrencyDTO, Definition, DexSymbols, Tickers,
    };

    #[test]
    fn eq_same_type() {
        assert_eq!(
            CurrencyDTO::<SuperGroup>::from_currency_type::<SuperGroupTestC1>(),
            CurrencyDTO::<SuperGroup>::from_currency_type::<SuperGroupTestC1>()
        );

        assert_ne!(
            CurrencyDTO::<SuperGroup>::from_currency_type::<SuperGroupTestC1>(),
            CurrencyDTO::<SuperGroup>::from_currency_type::<SuperGroupTestC2>()
        );
    }

    #[test]
    fn into_currency_type() {
        type TheC = SuperGroupTestC1;
        type OtherC = SuperGroupTestC2;
        let c1 = CurrencyDTO::<SuperGroup>::from_currency_type::<TheC>();
        assert_eq!(
            Ok(true),
            c1.into_currency_type(test::Expect::<TheC, SuperGroup, SuperGroup>::default())
        );

        assert_eq!(
            Ok(false),
            c1.into_currency_type(test::Expect::<OtherC, SuperGroup, SuperGroup>::default())
        );
    }

    #[test]
    fn into_super_group() {
        let sub_currency = CurrencyDTO::<SubGroup>::from_currency_type::<SubGroupTestC1>();
        assert_eq!(
            CurrencyDTO::<SuperGroup>::from_currency_type::<SubGroupTestC1>(),
            sub_currency.into_super_group::<SuperGroup>()
        )
    }

    #[test]
    fn from_super_group() {
        assert_eq!(
            CurrencyDTO::<SubGroup>::from_currency_type::<SubGroupTestC1>(),
            CurrencyDTO::<SuperGroup>::from_currency_type::<SubGroupTestC1>(),
        );

        assert_eq!(
            CurrencyDTO::<<SubGroupTestC1 as Currency>::Group>::from_currency_type::<SubGroupTestC1>(
            ),
            CurrencyDTO::<SubGroup>::from_currency_type::<SubGroupTestC1>()
        );
    }

    #[test]
    fn eq_other_type() {
        assert_ne!(
            CurrencyDTO::<SuperGroup>::from_currency_type::<SuperGroupTestC1>(),
            CurrencyDTO::<SubGroup>::from_currency_type::<SubGroupTestC1>()
        );
    }

    #[test]
    fn to_string() {
        assert_eq!(
            CurrencyDTO::<<SubGroupTestC1 as Currency>::Group>::from_currency_type::<SubGroupTestC1>().to_string(),
            super::to_string::<SubGroupTestC1>()
        );

        assert_eq!(
            super::symbol::<SubGroupTestC1, Tickers::<<SubGroupTestC1 as Currency>::Group>>(),
            super::to_string::<SubGroupTestC1>()
        );
    }

    #[test]
    fn into_symbol() {
        type TheC = SuperGroupTestC1;
        type TheG = <TheC as Currency>::Group;

        assert_eq!(
            TheC::BANK_SYMBOL,
            CurrencyDTO::<SuperGroup>::from_currency_type::<TheC>()
                .into_symbol::<BankSymbols::<TheG>>()
        );
        assert_eq!(
            TheC::DEX_SYMBOL,
            CurrencyDTO::<SuperGroup>::from_currency_type::<TheC>()
                .into_symbol::<DexSymbols::<TheG>>()
        );
        assert_eq!(
            TheC::TICKER,
            CurrencyDTO::<SuperGroup>::from_currency_type::<TheC>()
                .into_symbol::<Tickers::<TheG>>()
        );

        let c = CurrencyDTO::<SuperGroup>::from_currency_type::<TheC>();
        assert_eq!(c.to_string(), c.into_symbol::<Tickers::<TheG>>());
    }
}
