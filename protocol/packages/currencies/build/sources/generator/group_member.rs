use super::Generator;

pub(in super::super) trait GroupMember<
    'dex_currencies,
    'dex_currency_ticker,
    'dex_currency_definition,
> where
    'dex_currency_ticker: 'dex_currencies,
    'dex_currency_definition: 'dex_currencies,
{
    fn variant<'name>(&self, resolved_name: &'name str) -> impl IntoIterator<Item = &'name str>;

    fn first<'name>(&self, resolved_name: &'name str) -> impl IntoIterator<Item = &'name str>;

    fn next<'name>(
        &self,
        resolved_name: &'name str,
    ) -> Entry<
        impl IntoIterator<Item = &'name str>,
        impl IntoIterator<Item = &'name str>,
        impl IntoIterator<Item = &'name str>,
    >;

    fn filter_map<'name>(&self, resolved_name: &'name str) -> impl IntoIterator<Item = &'name str>;

    fn find_map<'name>(&self, resolved_name: &'name str) -> impl IntoIterator<Item = &'name str>;
}

impl<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition, const PAIRS_GROUP: bool>
    GroupMember<'dex_currencies, 'dex_currency_ticker, 'dex_currency_definition>
    for Generator<
        '_,
        '_,
        '_,
        'dex_currencies,
        'dex_currency_ticker,
        'dex_currency_definition,
        PAIRS_GROUP,
    >
{
    fn variant<'name>(&self, resolved_name: &'name str) -> impl IntoIterator<Item = &'name str> {
        [
            "
    ",
            resolved_name,
            ",",
        ]
    }

    fn first<'name>(&self, resolved_name: &'name str) -> impl IntoIterator<Item = &'name str> {
        ["Some(Self::", resolved_name, ")"]
    }

    #[inline]
    fn next<'name>(
        &self,
        resolved_name: &'name str,
    ) -> Entry<
        impl IntoIterator<Item = &'name str>,
        impl IntoIterator<Item = &'name str>,
        impl IntoIterator<Item = &'name str>,
    > {
        Entry {
            head: [
                "
            Self::",
                resolved_name,
                " => ",
            ],
            middle: [
                "Some(Self::",
                resolved_name,
                "),
            Self::",
                resolved_name,
                " => ",
            ],
            tail: ["None,
        "],
        }
    }

    fn filter_map<'name>(&self, resolved_name: &'name str) -> impl IntoIterator<Item = &'name str> {
        [
            "
            Self::",
            resolved_name,
            " => filter_map.on::<self::definitions::",
            resolved_name,
            ">(<self::definitions::",
            resolved_name,
            " as currency::CurrencyDef>::dto()),",
        ]
    }

    fn find_map<'name>(&self, resolved_name: &'name str) -> impl IntoIterator<Item = &'name str> {
        [
            "
            Self::",
            resolved_name,
            " => find_map.on::<self::definitions::",
            resolved_name,
            ">(<self::definitions::",
            resolved_name,
            " as currency::CurrencyDef>::dto()),",
        ]
    }
}

pub(in super::super) struct Entry<Head, Middle, Tail> {
    pub head: Head,
    pub middle: Middle,
    pub tail: Tail,
}
