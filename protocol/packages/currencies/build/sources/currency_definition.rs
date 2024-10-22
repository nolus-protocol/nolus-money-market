use std::{borrow::Cow, iter};

use topology::CurrencyDefinition;

pub(super) fn currency_definition<'r>(
    name: &'r str,
    ticker: &'r str,
    currency: &'r CurrencyDefinition,
) -> impl Iterator<Item = Cow<'r, str>> + use<'r> {
    [
        r#"
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize,
    serde::Deserialize, sdk::schemars::JsonSchema,
)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
#[schemars(crate = "sdk::schemars")]
pub struct "#,
        name,
        r#"(currency::CurrencyDTO<super::Group>);

impl currency::CurrencyDef for "#,
        name,
        r#" {
    type Group = super::Group;

    fn definition() -> &'static Self {
        const {
            &Self(currency::CurrencyDTO::new(
                const {
                    &currency::Definition::new(
                        ""#,
        ticker,
        r#"",
                        // "#,
        currency.host().path(),
        r#"
                        ""#,
        currency.host().symbol(),
        r#"",
                        // "#,
        currency.dex().path(),
        r#"
                        ""#,
        currency.dex().symbol(),
        r#"",
                        "#,
    ]
    .into_iter()
    .map(Cow::Borrowed)
    .chain(iter::once(Cow::Owned(
        currency.decimal_digits().to_string(),
    )))
    .chain(iter::once(
        const {
            Cow::Borrowed(
                r#",
                    )
                },
            ))
        }
    }

    fn dto(&self) -> &currency::CurrencyDTO<Self::Group> {
        &self.0
    }
}
"#,
            )
        },
    ))
}
