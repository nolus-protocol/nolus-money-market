use std::{
    fmt::{Debug, Display, Formatter},
    marker::PhantomData,
};

use sdk::schemars::{gen::SchemaGenerator, schema::Schema, JsonSchema};
use serde::{Deserialize, Serialize};

use crate::{
    definition::DefinitionRef,
    error::{Error, Result},
    group::MemberOf,
    CurrencyDef, Group, GroupVisit as _, MaybeAnyVisitResult, Symbol, SymbolSlice, SymbolStatic,
    Tickers, TypeMatcher,
};

use super::{AnyVisitor, AnyVisitorResult};

mod unchecked;

/// Data-Transferable currency belonging to a group
///
/// This is a value type designed for efficient representation, data transfer and storage.
/// `GroupMember` specifies which currencies are valid instances of this type.
#[derive(Copy, Clone, Debug, Eq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "unchecked::TickerDTO", into = "unchecked::TickerDTO")]
pub struct CurrencyDTO<G>
where
    G: Group,
{
    def: DefinitionRef,
    _host_group: PhantomData<G>,
}

impl<G> CurrencyDTO<G>
where
    G: Group,
{
    pub const fn new(def: DefinitionRef) -> Self {
        Self {
            def,
            _host_group: PhantomData,
        }
    }

    pub fn may_into_currency_type<SubG, V>(self, visitor: V) -> MaybeAnyVisitResult<SubG, V>
    where
        SubG: Group + MemberOf<G>,
        V: AnyVisitor<SubG, VisitorG = G>,
    {
        SubG::maybe_visit_super_visitor(&TypeMatcher::new(self.def), visitor)
    }

    pub fn into_currency_super_group_type<TopG, V>(self, visitor: V) -> AnyVisitorResult<G, V>
    where
        TopG: Group,
        G: MemberOf<TopG>,
        V: AnyVisitor<G, VisitorG = TopG>,
    {
        G::maybe_visit_super_visitor(&TypeMatcher::new(self.def), visitor)
            .unwrap_or_else(|_| self.unexpected::<V>())
    }

    pub fn into_currency_type<V>(self, visitor: V) -> AnyVisitorResult<G, V>
    where
        V: AnyVisitor<G, VisitorG = G>,
    {
        G::maybe_visit(&TypeMatcher::new(self.def), visitor)
            .unwrap_or_else(|_| self.unexpected::<V>())
    }

    pub fn into_super_group<SuperG>(self) -> CurrencyDTO<SuperG>
    where
        SuperG: Group,
        G: MemberOf<SuperG>,
    {
        CurrencyDTO::<SuperG> {
            def: self.def,
            _host_group: PhantomData,
        }
    }

    pub fn definition(&self) -> DefinitionRef {
        self.def
    }

    pub fn into_symbol<S>(self) -> SymbolStatic
    where
        S: Symbol,
    {
        S::symbol(self.def)
    }

    pub fn of_currency<SubG>(&self, def: &CurrencyDTO<SubG>) -> Result<()>
    where
        SubG: Group + MemberOf<G>,
    {
        if self == def {
            Ok(())
        } else {
            Err(Error::currency_mismatch(self, def))
        }
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn from_symbol_testing<S>(symbol: &SymbolSlice) -> Result<CurrencyDTO<G>>
    where
        S: Symbol<Group = G>,
    {
        Self::from_symbol::<S>(symbol)
    }

    fn from_symbol<S>(symbol: &SymbolSlice) -> Result<CurrencyDTO<G>>
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

            fn on<C>(self, def: &C) -> AnyVisitorResult<G, Self>
            where
                C: CurrencyDef,
                C::Group: MemberOf<G>,
            {
                Ok(def.dto().into_super_group::<G>())
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
        self.def.eq(other.def)
    }
}

/// Prepare a human-friendly representation of a currency
pub fn to_string<C>(def: &C) -> SymbolStatic
where
    C: CurrencyDef,
{
    Tickers::<C::Group>::symbol(def.dto().definition())
}

impl<G> Display for CurrencyDTO<G>
where
    G: Group,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        unchecked::TickerDTO::from(*self).fmt(f)
    }
}

impl<G> JsonSchema for CurrencyDTO<G>
where
    G: Group,
{
    fn schema_name() -> String {
        unchecked::TickerDTO::schema_name()
    }

    fn json_schema(gen: &mut SchemaGenerator) -> Schema {
        unchecked::TickerDTO::json_schema(gen)
    }
}

#[cfg(test)]
mod test {

    use crate::{
        test::{
            self, SubGroup, SubGroupTestC10, SuperGroup, SuperGroupTestC1, TESTC1, TESTC10,
            TESTC10_DEFINITION, TESTC1_DEFINITION, TESTC2, TESTC2_DEFINITION,
        },
        BankSymbols, CurrencyDTO, CurrencyDef, DexSymbols, Tickers,
    };

    #[test]
    fn eq_same_type() {
        assert_eq!(
            CurrencyDTO::<SuperGroup>::new(&TESTC1_DEFINITION),
            CurrencyDTO::<SuperGroup>::new(&TESTC1_DEFINITION)
        );

        assert_ne!(
            CurrencyDTO::<SuperGroup>::new(&TESTC1_DEFINITION),
            CurrencyDTO::<SuperGroup>::new(&TESTC2_DEFINITION)
        );
    }

    #[test]
    fn into_currency_type() {
        let c1 = CurrencyDTO::<SuperGroup>::new(&TESTC1_DEFINITION);
        assert_eq!(
            Ok(true),
            c1.into_currency_type(test::Expect::<_, SuperGroup, SuperGroup>::new(&TESTC1))
        );

        assert_eq!(
            Ok(false),
            c1.into_currency_type(test::Expect::<_, SuperGroup, SuperGroup>::new(&TESTC2))
        );
    }

    #[test]
    fn into_super_group() {
        let sub_currency = CurrencyDTO::<SubGroup>::new(&TESTC10_DEFINITION);
        assert_eq!(
            CurrencyDTO::<SuperGroup>::new(&TESTC10_DEFINITION),
            sub_currency.into_super_group::<SuperGroup>()
        )
    }

    #[test]
    fn from_super_group() {
        assert_eq!(
            CurrencyDTO::<SubGroup>::new(&TESTC10_DEFINITION),
            CurrencyDTO::<SuperGroup>::new(&TESTC10_DEFINITION),
        );

        assert_eq!(
            CurrencyDTO::<<SubGroupTestC10 as CurrencyDef>::Group>::new(&TESTC10_DEFINITION),
            CurrencyDTO::<SubGroup>::new(&TESTC10_DEFINITION)
        );
    }

    #[test]
    fn eq_other_type() {
        assert_ne!(
            CurrencyDTO::<SuperGroup>::new(&TESTC1_DEFINITION),
            CurrencyDTO::<SubGroup>::new(&TESTC10_DEFINITION)
        );
    }

    #[test]
    fn to_string() {
        assert_eq!(
            CurrencyDTO::<<SubGroupTestC10 as CurrencyDef>::Group>::new(&TESTC10_DEFINITION)
                .to_string(),
            super::to_string(&TESTC10)
        );
    }

    #[test]
    fn into_symbol() {
        type TheC = SuperGroupTestC1;
        type TheG = <TheC as CurrencyDef>::Group;

        assert_eq!(
            TESTC1.dto().definition().bank_symbol,
            TESTC1.dto().into_symbol::<BankSymbols::<TheG>>()
        );
        assert_eq!(
            TESTC1.dto().definition().dex_symbol,
            TESTC1.dto().into_symbol::<DexSymbols::<TheG>>()
        );
        assert_eq!(
            TESTC1.dto().definition().ticker,
            TESTC1.dto().into_symbol::<Tickers::<TheG>>()
        );

        let c = CurrencyDTO::<SuperGroup>::new(&TESTC1_DEFINITION);
        assert_eq!(c.to_string(), c.into_symbol::<Tickers::<TheG>>());
    }
}
