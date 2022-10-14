use finance::{
    coin::{Amount, Coin, CoinDTO},
    currency::{Currency, Symbol},
    percent::Percent,
};
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Env, Event, Timestamp},
};

use crate::batch::Batch;

pub trait Emit
where
    Self: Sized,
{
    fn emit<K, V>(self, event_key: K, event_value: V) -> Self
    where
        K: Into<String>,
        V: Into<String>;

    /// Specialization of [`emit`](Self::emit) for [`Timestamp`].
    fn emit_timestamp<K>(self, event_key: K, timestamp: &Timestamp) -> Self
    where
        K: Into<String>,
    {
        self.emit_to_string_value(event_key, timestamp.nanos())
    }

    /// Specialization of [`emit`](Self::emit) for values implementing [`ToString`].
    fn emit_to_string_value<K, V>(self, event_key: K, value: V) -> Self
    where
        K: Into<String>,
        V: ToString,
    {
        self.emit(event_key, value.to_string())
    }

    /// Specialization of [`emit`](Self::emit) for [`Coin`]'s amount.
    fn emit_coin_amount<K, A>(self, event_key: K, coin_amount: A) -> Self
    where
        K: Into<String>,
        A: Into<Amount>,
    {
        self.emit_to_string_value(event_key, coin_amount.into())
    }

    /// Specialization of [`emit`](Self::emit) for [`Currency`] implementors.
    fn emit_currency<K, C>(self, event_key: K) -> Self
    where
        K: Into<String>,
        C: Currency,
    {
        self.emit_currency_symbol(event_key, C::SYMBOL)
    }

    /// Specialization of [`emit`](Self::emit) for [`Currency`]'s symbol.
    fn emit_currency_symbol<K>(self, event_key: K, currency_symbol: Symbol) -> Self
    where
        K: Into<String>,
    {
        self.emit(event_key, currency_symbol)
    }

    /// Specialization of [`emit`](Self::emit) for [`Percent`]'s amount in [`Units`](finance::percent::Units).
    fn emit_percent_amount<K>(self, event_key: K, percent: Percent) -> Self
    where
        K: Into<String>,
    {
        self.emit_to_string_value(event_key, percent.units())
    }

    fn emit_coin<K, C>(self, event_key: K, coin: Coin<C>) -> Self
    where
        K: Into<String>,
        C: Currency,
    {
        self.emit_coin_dto(event_key, coin.into())
    }

    fn emit_coin_dto<K>(self, event_key: K, coin: CoinDTO) -> Self
    where
        K: Into<String>,
    {
        let key = event_key.into();
        let amount_key = key.clone() + "-amount";
        let symbol_key = key + "-symbol";

        self.emit_coin_amount(amount_key, coin.amount())
            .emit_currency_symbol(symbol_key, coin.symbol())
    }

    fn emit_tx_info(self, env: &Env) -> Self {
        self.emit_to_string_value("height", env.block.height)
            .emit_timestamp("at", &env.block.time)
            .emit_to_string_value(
                "idx",
                env.transaction
                    .as_ref()
                    .map(|transaction| transaction.index)
                    .expect("transaction index should be present"),
            )
    }
}

pub struct Emitter {
    batch: Batch,
    event: Event,
}

impl Emitter {
    pub(crate) fn new<T>(batch: Batch, event_type: T) -> Self
    where
        T: Into<String>,
    {
        Self {
            batch,
            event: Event::new(event_type.into()),
        }
    }
}

impl From<Emitter> for Response
where
    Batch: Into<Response>,
{
    fn from(emitter: Emitter) -> Self {
        Response::from(emitter.batch).add_event(emitter.event)
    }
}

impl Emit for Emitter {
    fn emit<K, V>(mut self, event_key: K, event_value: V) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.event = self.event.add_attribute(event_key, event_value);

        self
    }
}
