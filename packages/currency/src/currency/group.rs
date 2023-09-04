use super::{AnyVisitor, AnyVisitorResult, Matcher, MatcherExt, SymbolStatic, TickerMatcher};

pub trait Group: PartialEq + Sized {
    const DESCR: SymbolStatic;

    fn get_from<M: MatcherExt>(matcher: M, field_value: &M::FieldType) -> Option<Self>;

    fn visit<V: AnyVisitor>(&self, visitor: V) -> AnyVisitorResult<V>;
}

pub trait GroupExt: Group {
    fn get_from_ticker(ticker: &<TickerMatcher as Matcher>::FieldType) -> Option<Self> {
        Self::get_from(TickerMatcher, ticker)
    }

    fn get_from_bank_symbol(bank_symbol: &<TickerMatcher as Matcher>::FieldType) -> Option<Self> {
        Self::get_from(TickerMatcher, bank_symbol)
    }

    fn get_from_dex_symbol(dex_symbol: &<TickerMatcher as Matcher>::FieldType) -> Option<Self> {
        Self::get_from(TickerMatcher, dex_symbol)
    }
}

impl<T> GroupExt for T where T: Group {}

pub type MaybeAnyVisitResult<V> = Result<AnyVisitorResult<V>, V>;

#[macro_export]
macro_rules! impl_group_variants_from {
    ($group:ident = [$($currency:ident),+ $(,)?]) => {
        $(
            impl ::core::convert::From<$currency> for $group {
                fn from(currency: $currency) -> Self {
                    Self::$currency(currency)
                }
            }
        )+
    };
}

#[macro_export]
macro_rules! impl_group_for_prime_group {
    ($group:ident = ($id:literal) [$first_currency:ident $(, $other_currencies:ident)* $(,)?]) => {
        impl $crate::currency::Group for $group {
            const DESCR: $crate::currency::SymbolStatic = $id;

            fn get_from<M: $crate::currency::MatcherExt>(matcher: M, field_value: &M::FieldType) -> Option<Self> {
                matcher
                    .match_field_and_into::<$first_currency, _>(field_value)
                    $(
                        .or_else(|| matcher.match_field_and_into::<$other_currencies, _>(field_value))
                    )*
            }

            fn visit<V: $crate::currency::AnyVisitor>(&self, visitor: V) -> crate::AnyVisitorResult<V> {
                match self {
                    $group::$first_currency(_) => visitor.on::<$first_currency>(),
                    $(
                        $group::$other_currencies(_) => visitor.on::<$other_currencies>(),
                    )*
                }
            }
        }
    };
}

#[macro_export]
macro_rules! define_prime_group {
    ($group:ident = ($id:literal) [$($currency:ident),+ $(,)?]) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, ::serde::Serialize, ::serde::Deserialize, ::sdk::schemars::JsonSchema)]
        #[serde(deny_unknown_fields, rename_all = "snake_case")]
        pub enum $group {
            $($currency($currency)),+
        }

        $crate::impl_group_for_prime_group! {
            $group = ($id)[$($currency),+]
        }

        $crate::impl_group_variants_from! {
            $group = [$($currency),+]
        }
    };
}
