use std::io::{Error as IOError, Write};

pub(super) struct Group {
    name: String,
    filename: String,
    currencies: Vec<String>,
}

impl Group {
    pub(super) fn new(name: &str, currencies: Vec<String>) -> Self {
        Self {
            name: name
                .get(..1)
                .expect("Empty group name encountered!")
                .to_ascii_uppercase()
                + &name[1..].to_ascii_lowercase(),
            filename: String::from(name).to_ascii_lowercase(),
            currencies,
        }
    }

    pub(super) fn generate<W>(
        &self,
        template: &[Token],
        currencies_module: &'static str,
        mut writer: W,
    ) -> Result<(), IOError>
    where
        W: Write,
    {
        let currencies_module = currencies_module.as_bytes();

        for token in template {
            match token {
                Token::Raw(raw) => writer.write_all(raw.as_bytes())?,
                Token::Name => writer.write_all(self.name.as_bytes())?,
                Token::CurrenciesModule => writer.write_all(currencies_module)?,
                Token::ForEachCurrency(template) => {
                    for currency in &self.currencies {
                        let currency = currency.as_bytes();

                        for token in template {
                            writer.write_all(match token {
                                RepeatSequenceToken::Raw(raw) => raw.as_bytes(),
                                RepeatSequenceToken::Name => self.name.as_bytes(),
                                RepeatSequenceToken::CurrenciesModule => currencies_module,
                                RepeatSequenceToken::Currency => currency,
                            })?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub(super) fn filename(&self) -> &String {
        &self.filename
    }
}

#[derive(Debug)]
pub(super) enum Token {
    Raw(&'static str),
    Name,
    CurrenciesModule,
    ForEachCurrency(Vec<RepeatSequenceToken>),
}

#[derive(Debug)]
pub(super) enum RepeatSequenceToken {
    Raw(&'static str),
    Name,
    CurrenciesModule,
    Currency,
}
