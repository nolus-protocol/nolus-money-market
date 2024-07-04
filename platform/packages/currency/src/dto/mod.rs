use std::{
    any::TypeId,
    fmt::{Debug, Display, Formatter},
    marker::PhantomData,
};

use sdk::schemars::{gen::SchemaGenerator, schema::Schema, JsonSchema};
use serde::{Deserialize, Serialize};

use crate::{
    group::MemberOf, never::{self, Never}, Currency, Definition, Group, Symbol, SymbolStatic, Tickers, TypeMatcher
};

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
        let id = TypeId::of::<C>();
        Self {
            id,
            _group_member: PhantomData,
        }
    }

    pub fn into_currency_type<V>(self, visitor: V) -> AnyVisitorResult<V>
    where
        V: AnyVisitor<VisitedG = G>,
        G: MemberOf<V::VisitedG>,
    {
        G::maybe_visit(&TypeMatcher::new(self.id), visitor).unwrap_or_else(|_| {
            panic!(
                r#"Found an invalid currency instance! "{:?}" did not match "{}" !"#,
                self,
                G::DESCR
            )
        })
    }

    pub fn into_symbol<S>(self) -> SymbolStatic
    where
        S: Symbol,
    {
        struct SymbolRetriever<G, S> {
            visited_g: PhantomData<G>,
            symbol: PhantomData<S>,
        }

        impl<G, S> AnyVisitor for SymbolRetriever<G, S>
        where
            G: Group,
            S: Symbol,
        {
            type VisitedG = G;

            type Output = SymbolStatic;

            type Error = Never;

            fn on<C>(self) -> AnyVisitorResult<Self>
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
}

pub fn symbol<C, S>() -> SymbolStatic
where
    C: Currency,
    S: Symbol,
{
    CurrencyDTO::<C::Group>::from_currency_type::<C>().into_symbol::<S>()
}

/// Prepare a human-friendly representation of a currency
pub fn to_string<C>() -> String
where
    C: Currency,
{
    symbol::<C, Tickers>().to_owned()
}

impl<G> Display for CurrencyDTO<G>
where
    G: Group,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", unchecked::CurrencyDTO::from(*self)))
    }
}

impl<G, GSelf> PartialEq<CurrencyDTO<G>> for CurrencyDTO<GSelf>
where
    G: Group,
    GSelf: Group,
{
    fn eq(&self, other: &CurrencyDTO<G>) -> bool {
        self.id == other.id
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
            c1.into_currency_type(test::Expect::<TheC, SuperGroup>::new())
        );

        assert_eq!(
            Ok(false),
            c1.into_currency_type(test::Expect::<OtherC, SuperGroup>::new())
        );
    }

    #[test]
    fn from_super_group() {
        assert_eq!(
            CurrencyDTO::<SuperGroup>::from_currency_type::<SubGroupTestC1>(),
            CurrencyDTO::<SubGroup>::from_currency_type::<SubGroupTestC1>()
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
            super::symbol::<SubGroupTestC1, Tickers>(),
            super::to_string::<SubGroupTestC1>()
        );
    }

    #[test]
    fn into_symbol() {
        type TheC = SuperGroupTestC1;
        assert_eq!(
            TheC::BANK_SYMBOL,
            CurrencyDTO::<SuperGroup>::from_currency_type::<TheC>().into_symbol::<BankSymbols>()
        );
        assert_eq!(
            TheC::DEX_SYMBOL,
            CurrencyDTO::<SuperGroup>::from_currency_type::<TheC>().into_symbol::<DexSymbols>()
        );
        assert_eq!(
            TheC::TICKER,
            CurrencyDTO::<SuperGroup>::from_currency_type::<TheC>().into_symbol::<Tickers>()
        );

        let c = CurrencyDTO::<SuperGroup>::from_currency_type::<TheC>();
        assert_eq!(c.to_string(), c.into_symbol::<Tickers>());
    }
}
