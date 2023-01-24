use std::borrow::Cow;

use heck::ToSnakeCase;

pub trait ModuleName {
    fn module_name(&self) -> String;
}

#[derive(Debug, Clone)]
pub struct Currency {
    struct_name: String,
    ticker: String,
    friendly_name: String,
    symbol: String,
    ibc_route: Vec<String>,
}

impl Currency {
    pub const fn new(
        struct_name: String,
        ticker: String,
        friendly_name: String,
        symbol: String,
        ibc_route: Vec<String>,
    ) -> Self {
        Self {
            struct_name,
            ticker,
            friendly_name,
            symbol,
            ibc_route,
        }
    }
}

impl ModuleName for Currency {
    fn module_name(&self) -> String {
        self.ticker.to_snake_case()
    }
}

#[derive(Debug, Clone)]
pub struct CurrencyWithModule {
    module: String,
    currency: Currency,
}

impl CurrencyWithModule {
    pub const fn new(module: String, currency: Currency) -> Self {
        Self { module, currency }
    }

    fn module(&self) -> &str {
        &self.module
    }

    fn currency(&self) -> &Currency {
        &self.currency
    }
}

#[derive(Debug, Clone)]
pub struct Group {
    struct_name: String,
    friendly_name: String,
    currencies: Vec<CurrencyWithModule>,
}

impl Group {
    pub const fn new(
        struct_name: String,
        friendly_name: String,
        currencies: Vec<CurrencyWithModule>,
    ) -> Self {
        Self {
            struct_name,
            friendly_name,
            currencies,
        }
    }
}

impl ModuleName for Group {
    fn module_name(&self) -> String {
        self.friendly_name.to_snake_case()
    }
}

pub trait Template<const ARRAY_SIZE: usize> {
    type SubstitutionData;

    fn substitute(currency: &Self::SubstitutionData) -> [Cow<str>; ARRAY_SIZE];
}

pub struct CurrencyTemplate;

impl CurrencyTemplate {
    pub const ARRAY_SIZE: usize = currency_template::CURRENCY_TEMPLATE.len();
}

impl Template<{ Self::ARRAY_SIZE }> for CurrencyTemplate {
    type SubstitutionData = Currency;

    fn substitute(currency: &Self::SubstitutionData) -> [Cow<str>; Self::ARRAY_SIZE] {
        std::array::from_fn(|index| {
            currency_template::CURRENCY_TEMPLATE[index].substitute(currency)
        })
    }
}

pub struct GroupTemplate;

impl GroupTemplate {
    pub const ARRAY_SIZE: usize = group_template::GROUP_TEMPLATE.len();
}

impl Template<{ Self::ARRAY_SIZE }> for GroupTemplate {
    type SubstitutionData = Group;

    fn substitute(group: &Self::SubstitutionData) -> [Cow<str>; Self::ARRAY_SIZE] {
        std::array::from_fn(|index| group_template::GROUP_TEMPLATE[index].substitute(group))
    }
}

mod currency_template {
    use std::borrow::Cow;

    use super::Currency;

    pub trait Token {
        fn substitute<'r>(&'r self, currency: &'r Currency) -> Cow<'r, str>;
    }

    pub const CURRENCY_TEMPLATE: &[&dyn Token] = {
        impl<'str> Token for &'str str {
            fn substitute<'r>(&'r self, _: &'r Currency) -> Cow<'r, str> {
                Cow::Borrowed(self)
            }
        }

        pub struct StructName;

        impl Token for StructName {
            fn substitute<'r>(&'r self, currency: &'r Currency) -> Cow<'r, str> {
                Cow::Borrowed(currency.struct_name.as_str())
            }
        }

        pub struct Ticker;

        impl Token for Ticker {
            fn substitute<'r>(&'r self, currency: &'r Currency) -> Cow<'r, str> {
                Cow::Borrowed(currency.ticker.as_str())
            }
        }

        pub struct FriendlyName;

        impl Token for FriendlyName {
            fn substitute<'r>(&'r self, currency: &'r Currency) -> Cow<'r, str> {
                Cow::Borrowed(currency.friendly_name.as_str())
            }
        }

        pub struct Symbol;

        impl Token for Symbol {
            fn substitute<'r>(&'r self, currency: &'r Currency) -> Cow<'r, str> {
                Cow::Borrowed(currency.symbol.as_str())
            }
        }

        fn digest_ibc_route(
            ibc_route: &[String],
            symbol: &str,
        ) -> sha2::digest::Output<sha2::Sha256> {
            use sha2::{
                digest::{Digest as _, FixedOutput as _},
                Sha256,
            };

            let mut hasher: Sha256 = Sha256::new();

            hasher.update({
                const TRANSFER: &str = "transfer/";

                let mut aggregated_route: String = String::with_capacity(
                    ((TRANSFER.len() + 1 /* Accommodating for the slash after the IBC route segments */)
                        * ibc_route.len())
                        + ibc_route.iter().map(String::len).sum::<usize>()
                        + symbol.len(),
                );

                for segment in ibc_route {
                    aggregated_route.push_str(TRANSFER);
                    aggregated_route.push_str(segment.as_ref());
                    aggregated_route.push('/');
                }

                aggregated_route.push_str(symbol);

                aggregated_route
            }.as_bytes());

            hasher.finalize_fixed()
        }

        fn aggregate_ibc_route(ibc_route: &[String], symbol: &str) -> String {
            use sha2::{digest::Output, Sha256};

            let raw_ibc_symbol: Output<Sha256> = digest_ibc_route(ibc_route, symbol);

            let mut ibc_symbol: String = String::with_capacity(raw_ibc_symbol.len() * 2);

            raw_ibc_symbol.into_iter().for_each(|byte| {
                fn to_hex(byte: u8) -> char {
                    match byte {
                        hex if hex < 10 => char::from(b'0' + hex),
                        hex => char::from(b'A' + (hex - 10)),
                    }
                }

                ibc_symbol.push(to_hex(byte >> 4));
                ibc_symbol.push(to_hex(byte & 15));
            });

            ibc_symbol
        }

        pub struct BankSymbol;

        impl Token for BankSymbol {
            fn substitute<'r>(&'r self, currency: &'r Currency) -> Cow<'r, str> {
                Cow::Owned(aggregate_ibc_route(
                    currency.ibc_route.as_slice(),
                    &currency.symbol,
                ))
            }
        }

        pub struct DexSymbol;

        impl Token for DexSymbol {
            fn substitute<'r>(&'r self, currency: &'r Currency) -> Cow<'r, str> {
                Cow::Owned(aggregate_ibc_route(
                    currency.ibc_route.get(1..).unwrap_or(&[]),
                    &currency.symbol,
                ))
            }
        }

        &include!("../templates/currency.template.txt")
    };
}

mod group_template {
    use std::borrow::Cow;

    use super::{CurrencyWithModule, Group};

    pub trait Token {
        fn substitute<'r>(&'r self, group: &'r Group) -> Cow<'r, str>;
    }

    pub const GROUP_TEMPLATE: &[&dyn Token] = {
        impl<'str> Token for &'str str {
            fn substitute<'r>(&'r self, _: &'r Group) -> Cow<'r, str> {
                Cow::Borrowed(self)
            }
        }

        pub struct StructName;

        impl Token for StructName {
            fn substitute<'r>(&'r self, group: &'r Group) -> Cow<'r, str> {
                Cow::Borrowed(group.struct_name.as_str())
            }
        }

        pub struct FriendlyName;

        impl Token for FriendlyName {
            fn substitute<'r>(&'r self, group: &'r Group) -> Cow<'r, str> {
                Cow::Borrowed(group.friendly_name.as_str())
            }
        }

        pub struct CurrenciesModule;

        impl Token for CurrenciesModule {
            fn substitute<'r>(&'r self, _: &'r Group) -> Cow<'r, str> {
                Cow::Borrowed("currencies")
            }
        }

        pub struct ForEachCurrency {
            pattern: &'static [&'static dyn HybridToken],
            delimiter: &'static str,
        }

        impl Token for ForEachCurrency {
            fn substitute<'r>(&'r self, group: &'r Group) -> Cow<'r, str> {
                Cow::Owned(
                    group
                        .currencies
                        .iter()
                        .map(|currency| {
                            self.pattern
                                .iter()
                                .map(|token| token.substitute(group, currency))
                                .fold(String::new(), |mut accumulator, substitution| {
                                    accumulator.push_str(&substitution);

                                    accumulator
                                })
                        })
                        .fold(String::new(), {
                            let mut first: bool = true;

                            move |mut accumulator, entry| {
                                if first {
                                    first = false;
                                } else {
                                    accumulator.push_str(self.delimiter);
                                }

                                accumulator.push_str(&entry);

                                accumulator
                            }
                        }),
                )
            }
        }

        pub trait HybridToken {
            fn substitute<'r>(
                &'r self,
                group: &'r Group,
                currency: &'r CurrencyWithModule,
            ) -> Cow<'r, str>;
        }

        impl<T> HybridToken for T
        where
            T: Token,
        {
            fn substitute<'r>(
                &'r self,
                group: &'r Group,
                _: &'r CurrencyWithModule,
            ) -> Cow<'r, str> {
                self.substitute(group)
            }
        }

        pub struct CurrencyModule;

        impl HybridToken for CurrencyModule {
            fn substitute<'r>(
                &'r self,
                _: &'r Group,
                currency: &'r CurrencyWithModule,
            ) -> Cow<'r, str> {
                Cow::Borrowed(currency.module())
            }
        }

        pub struct CurrencyStructName;

        impl HybridToken for CurrencyStructName {
            fn substitute<'r>(
                &'r self,
                _: &'r Group,
                currency: &'r CurrencyWithModule,
            ) -> Cow<'r, str> {
                Cow::Borrowed(&currency.currency().struct_name)
            }
        }

        &include!("../templates/group.template.txt")
    };
}
