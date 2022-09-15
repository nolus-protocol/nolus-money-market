use serde::{de::DeserializeOwned, Serialize};

use crate::{
    coin::{Coin, CoinDTO},
    currency::{visit_any, AnyVisitor, Currency},
    price::Price,
};

use super::{PriceDTO, WithBase, WithPrice};

struct QuoteCVisitor<C, Cmd>
where
    C: Currency + Serialize + DeserializeOwned,
    Cmd: WithBase<C>,
{
    base: Coin<C>,
    quote_dto: CoinDTO,
    cmd: Cmd,
}

impl<C, Cmd> AnyVisitor for QuoteCVisitor<C, Cmd>
where
    C: Currency + Serialize + DeserializeOwned,
    Cmd: WithBase<C>,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn on<QuoteC>(self) -> Result<Self::Output, Self::Error>
    where
        QuoteC: Currency + Serialize + DeserializeOwned,
    {
        println!("-----------------------------------");
        println!("QuoteC  {}", QuoteC::SYMBOL.to_string());
        println!("QuoteC  {}", C::SYMBOL.to_string());

        println!("quote_dto  {}", self.quote_dto.symbol().to_string());

        println!("-----------------------------------");
        self.cmd.exec(Price::new(
            self.base,
            Coin::<QuoteC>::try_from(self.quote_dto)
                .expect("Got different currency in visitor! ===============>2"),
        ))
    }

    fn on_unknown(self) -> Result<Self::Output, Self::Error> {
        self.cmd.unknown()
    }
}

pub fn execute<Cmd, C>(price: PriceDTO, cmd: Cmd) -> Result<Cmd::Output, Cmd::Error>
where
    Cmd: WithBase<C>,
    C: Currency + Serialize + DeserializeOwned,
{
    println!("-----------------------------------");
    println!("price base {}", price.amount.symbol().clone().to_string());
    println!("price quote {}", price.amount_quote.symbol().to_string());
    println!("C  {}", C::SYMBOL.to_string());

    println!("-----------------------------------");
    visit_any(
        &price.amount_quote.symbol().clone(),
        QuoteCVisitor {
            base: Coin::<C>::try_from(price.amount)
                .expect("Got different currency in visitor! ===============>1"),
            quote_dto: price.amount_quote,
            cmd: cmd,
        },
    )
}
