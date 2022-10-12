use std::io::{Error as IOError, Write};

#[derive(Debug)]
pub(super) struct Group {
    name: String,
    filename: String,
    currencies: Vec<CurrencyTickerPair>,
}

impl Group {
    pub(super) fn new(name: &str, currencies: Vec<CurrencyTickerPair>) -> Self {
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
                        let currency = currency.normalized().as_bytes();

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

    pub(super) fn name(&self) -> &String {
        &self.name
    }

    pub(super) fn filename(&self) -> &String {
        &self.filename
    }

    pub(super) fn currencies(&self) -> &Vec<CurrencyTickerPair> {
        &self.currencies
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

#[derive(Debug)]
pub(super) struct CurrencyTickerPair {
    raw: String,
    normalized: String,
}

impl CurrencyTickerPair {
    pub(super) fn new(raw: String, normalized: String) -> Self {
        Self { raw, normalized }
    }
    pub fn raw(&self) -> &String {
        &self.raw
    }
    pub fn normalized(&self) -> &String {
        &self.normalized
    }
}
