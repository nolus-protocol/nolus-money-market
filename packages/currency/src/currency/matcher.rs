use std::marker::PhantomData;

use super::{Currency, SymbolStatic, SymbolUnsized};

pub trait ConstField {
    type Type;

    const VALUE: Self::Type;
}

pub struct Ticker<C: Currency>(PhantomData<C>);

impl<C: Currency> ConstField for Ticker<C> {
    type Type = SymbolStatic;

    const VALUE: Self::Type = C::TICKER;
}

pub struct BankSymbol<C: Currency>(PhantomData<C>);

impl<C: Currency> ConstField for BankSymbol<C> {
    type Type = SymbolStatic;

    const VALUE: Self::Type = C::BANK_SYMBOL;
}

pub struct DexSymbol<C: Currency>(PhantomData<C>);

impl<C: Currency> ConstField for DexSymbol<C> {
    type Type = SymbolStatic;

    const VALUE: Self::Type = C::DEX_SYMBOL;
}

pub trait Matcher: Copy {
    type FieldType: Eq + ?Sized + 'static;

    type ConstField<C: Currency>: ConstField<Type = &'static Self::FieldType>;
}

pub trait MatcherExt: Matcher {
    fn match_field<C>(&self, field_value: &Self::FieldType) -> Option<C>
    where
        C: Currency,
    {
        (field_value == <Self::ConstField<C> as ConstField>::VALUE).then(Default::default)
    }

    fn match_field_and_into<C, T>(&self, field_value: &Self::FieldType) -> Option<T>
    where
        C: Currency + Into<T>,
    {
        (field_value == <Self::ConstField<C> as ConstField>::VALUE).then(|| C::default().into())
    }
}

impl<T> MatcherExt for T where T: Matcher + ?Sized {}

#[derive(Clone, Copy)]
pub struct TickerMatcher;

impl Matcher for TickerMatcher {
    type FieldType = SymbolUnsized;

    type ConstField<C: Currency> = Ticker<C>;
}

#[derive(Clone, Copy)]
pub struct BankSymbolMatcher;

impl Matcher for BankSymbolMatcher {
    type FieldType = SymbolUnsized;

    type ConstField<C: Currency> = BankSymbol<C>;
}

#[derive(Clone, Copy)]
pub struct DexSymbolMatcher;

impl Matcher for DexSymbolMatcher {
    type FieldType = SymbolUnsized;

    type ConstField<C: Currency> = DexSymbol<C>;
}
