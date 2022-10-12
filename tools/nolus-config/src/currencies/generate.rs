use std::{
    collections::btree_map::{BTreeMap, Iter as BTreeMapIter},
    io::{Error as IOError, Write},
    slice::Iter,
};

use crate::{currencies::group::CurrencyTickerPair, CURRENCY_TEMPLATE, GROUP_TEMPLATE};

use super::{
    currency::{Currency, Token as CurrencyToken},
    group::{Group, RepeatSequenceToken as RepeatSequenceGroupToken, Token as GroupToken},
    Currencies,
};

type OuterTemplateFunctor<T> = fn(&'static str) -> Option<(&'static str, &'static str, T)>;

type InnerTemplateFunctor<T> =
    fn(&'static str, &'static str, T) -> Option<(&'static str, &'static str, T)>;

impl Currencies {
    pub fn generate(self, currencies_module: &'static str) -> GenerationResult {
        let mut currency_template = Vec::new();

        {
            let mut raw_template = CURRENCY_TEMPLATE;

            let functor: OuterTemplateFunctor<CurrencyToken> = |template| {
                let functor: InnerTemplateFunctor<CurrencyToken> =
                    |template, element, placeholder| {
                        template.find(element).map(|position| {
                            let (left, right) = template.split_at(position);

                            (left, &right[element.len()..], placeholder)
                        })
                    };

                functor(template, "#name#", CurrencyToken::Name)
                    .or_else(|| functor(template, "#ticker#", CurrencyToken::Ticker))
                    .or_else(|| {
                        functor(
                            template,
                            "#normalized_ticker#",
                            CurrencyToken::NormalizedTicker,
                        )
                    })
                    .or_else(|| functor(template, "#symbol#", CurrencyToken::Symbol))
            };

            while let Some((raw, rest, token)) = functor(raw_template) {
                raw_template = rest;

                if !raw.is_empty() {
                    currency_template.push(CurrencyToken::Raw(raw));
                }

                currency_template.push(token);
            }

            if !raw_template.is_empty() {
                currency_template.push(CurrencyToken::Raw(raw_template));
            }
        }

        let mut group_template = Vec::new();

        {
            let mut raw_template = GROUP_TEMPLATE;

            let functor: OuterTemplateFunctor<GroupToken> = |template| {
                let functor: InnerTemplateFunctor<GroupToken> = |template, element, placeholder| {
                    template
                        .find(element)
                        .and_then(|position| {
                            template[..position]
                                .find("#nane#")
                                .or_else(|| template[..position].find("#currencies_module#"))
                                .is_none()
                                .then_some(position)
                        })
                        .map(|position| {
                            let (left, right) = template.split_at(position);

                            (left, &right[element.len()..], placeholder)
                        })
                };

                template
                    .find('$')
                    .and_then(|left_position| {
                        if template[..left_position]
                            .find("#name#")
                            .or_else(|| template[..left_position].find("#currencies_module#"))
                            .is_some()
                        {
                            return None;
                        }

                        template
                            .get(left_position + 1..)?
                            .find('$')
                            .map(|right_position| {
                                (
                                    &template[..left_position],
                                    &template[left_position + right_position + 2 /* Two enclosing dollar signs */..],
                                    GroupToken::ForEachCurrency({
                                        let functor: OuterTemplateFunctor<
                                            RepeatSequenceGroupToken,
                                        > = |template| {
                                            let functor: InnerTemplateFunctor<
                                                RepeatSequenceGroupToken,
                                            > = |template, element, placeholder| {
                                                template.find(element).map(|position| {
                                                    let (left, right) = template.split_at(position);

                                                    (
                                                        left,
                                                        &right[element.len()..],
                                                        placeholder,
                                                    )
                                                })
                                            };

                                            functor(
                                                template,
                                                "#name#",
                                                RepeatSequenceGroupToken::Name,
                                            )
                                            .or_else(|| {
                                                functor(
                                                    template,
                                                    "#currencies_module#",
                                                    RepeatSequenceGroupToken::CurrenciesModule,
                                                )
                                            })
                                            .or_else(
                                                || {
                                                    functor(
                                                        template,
                                                        "#currency#",
                                                        RepeatSequenceGroupToken::Currency,
                                                    )
                                                },
                                            )
                                        };

                                        let mut raw_template =
                                            &template[left_position + 1..][..right_position];

                                        let mut group_template = Vec::new();

                                        while let Some((raw, rest, token)) = functor(raw_template) {
                                            raw_template = rest;

                                            if !raw.is_empty() {
                                                group_template
                                                    .push(RepeatSequenceGroupToken::Raw(raw));
                                            }

                                            group_template.push(token);
                                        }

                                        if !raw_template.is_empty() {
                                            group_template
                                                .push(RepeatSequenceGroupToken::Raw(raw_template));
                                        }

                                        group_template
                                    }),
                                )
                            })
                    })
                    .or_else(|| functor(template, "#name#", GroupToken::Name))
                    .or_else(|| {
                        functor(
                            template,
                            "#currencies_module#",
                            GroupToken::CurrenciesModule,
                        )
                    })
            };

            while let Some((raw, rest, token)) = functor(raw_template) {
                raw_template = rest;

                if !raw.is_empty() {
                    group_template.push(GroupToken::Raw(raw));
                }

                group_template.push(token);
            }

            if !raw_template.is_empty() {
                group_template.push(GroupToken::Raw(raw_template));
            }
        }

        let groups = self
            .currencies
            .iter()
            .fold::<BTreeMap<String, Vec<CurrencyTickerPair>>, _>(
                BTreeMap::new(),
                |mut groups, currency| {
                    currency.groups().iter().cloned().for_each(|group| {
                        groups
                            .entry(group)
                            .or_default()
                            .push(CurrencyTickerPair::new(
                                currency.ticker().clone(),
                                currency.normalized_ticker().clone(),
                            ))
                    });

                    groups
                },
            )
            .into_iter()
            .map(|(name, currencies)| Group::new(&name, currencies))
            .collect::<Vec<_>>();

        let currency_functor =
            |currency: Currency| (currency.ticker().to_ascii_lowercase(), currency);

        let currencies = self
            .currencies
            .into_iter()
            .map(currency_functor)
            .collect::<BTreeMap<_, _>>();

        if !currencies
            .values()
            .any(|currency| currency.ticker() == "NLS")
        {
            panic!("Nolus /NLS/ not defined!");
        }

        let lpns = groups
            .iter()
            .find(|group| group.name() == "Lpns")
            .expect("Liquidity provider's pool group /Lpns/ not defined!");

        currencies.values().for_each(|currency| {
            if currency.groups().iter().any(|group| ["Lease", "Payment"].contains(&group.as_str())) {
                assert_ne!(
                    currency.resolution_paths().len(),
                    0,
                    "\"{}\" doesn't define any resolution paths!",
                    currency.ticker()
                );
            }

            currency.resolution_paths().iter().for_each(|path| {
                assert!(
                    path.len() > 1,
                    "One of \"{}\"'s resolution paths doesn't contain at least two elements!",
                    currency.ticker()
                );

                if &path[0] != currency.ticker() {
                    panic!(
                        "One of \"{}\"'s resolution paths doesn't start with the currency itself!",
                        currency.ticker()
                    );
                }

                if !lpns.currencies().iter().map(|ticker_pair| ticker_pair.raw()).any(|ticker| ticker == &path[path.len() - 1]) {
                    panic!(
                        "One of \"{}\"'s resolution paths doesn't end with a currency from the Lpns group!",
                        currency.ticker()
                    );
                }
            })
        });

        GenerationResult {
            currencies: CurrencySources {
                template: currency_template,
                currencies,
            },
            groups: GroupSources {
                currencies_module,
                template: group_template,
                groups,
            },
        }
    }
}

pub struct GenerationResult {
    pub currencies: CurrencySources,
    pub groups: GroupSources,
}

pub struct CurrencySources {
    template: Vec<CurrencyToken>,
    currencies: BTreeMap<String, Currency>,
}

impl CurrencySources {
    pub fn iter(&self) -> CurrencySourcesIter {
        CurrencySourcesIter {
            template: self.template.as_slice(),
            currencies: self.currencies.iter(),
        }
    }
}

pub struct CurrencySourcesIter<'r> {
    template: &'r [CurrencyToken],
    currencies: BTreeMapIter<'r, String, Currency>,
}

impl<'r> Iterator for CurrencySourcesIter<'r> {
    type Item = CurrencyFilenameSource<'r>;

    fn next(&mut self) -> Option<Self::Item> {
        self.currencies
            .next()
            .map(|(filename, currency)| CurrencyFilenameSource {
                filename,
                template: self.template,
                currency,
            })
    }
}

pub struct CurrencyFilenameSource<'r> {
    filename: &'r String,
    template: &'r [CurrencyToken],
    currency: &'r Currency,
}

impl<'r> CurrencyFilenameSource<'r> {
    pub fn filename(&self) -> &str {
        self.filename.as_str()
    }

    pub fn generate_source<W>(&self, writer: W) -> Result<(), IOError>
    where
        W: Write,
    {
        self.currency.generate(self.template, writer)
    }
}

pub struct GroupSources {
    currencies_module: &'static str,
    template: Vec<GroupToken>,
    groups: Vec<Group>,
}

impl GroupSources {
    pub fn iter(&self) -> GroupsSourcesIter {
        GroupsSourcesIter {
            currencies_module: self.currencies_module,
            template: self.template.as_slice(),
            groups: self.groups.iter(),
        }
    }
}

pub struct GroupsSourcesIter<'r> {
    currencies_module: &'static str,
    template: &'r [GroupToken],
    groups: Iter<'r, Group>,
}

impl<'r> Iterator for GroupsSourcesIter<'r> {
    type Item = GroupFilenameSource<'r>;

    fn next(&mut self) -> Option<Self::Item> {
        self.groups.next().map(|group| GroupFilenameSource {
            currencies_module: self.currencies_module,
            template: self.template,
            group,
        })
    }
}

pub struct GroupFilenameSource<'r> {
    currencies_module: &'static str,
    template: &'r [GroupToken],
    group: &'r Group,
}

impl<'r> GroupFilenameSource<'r> {
    pub fn filename(&self) -> &str {
        self.group.filename().as_str()
    }

    pub fn generate_source<W>(&self, writer: W) -> Result<(), IOError>
    where
        W: Write,
    {
        self.group
            .generate(self.template, self.currencies_module, writer)
    }
}
