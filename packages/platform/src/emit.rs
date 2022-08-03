use cosmwasm_std::{Event, Response, Timestamp};
use finance::coin::{Amount, Coin};
use finance::currency::Currency;
use finance::percent::Percent;

use crate::batch::Batch;

pub trait Emit where Self: Sized {
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
    fn emit_coin_amount<K, C>(self, event_key: K, coin: Coin<C>) -> Self
        where
            K: Into<String>,
            C: Currency,
    {
        self.emit_to_string_value(event_key, Amount::from(coin))
    }

    /// Specialization of [`emit`](Self::emit) for [`Percent`]'s amount in [`Units`](finance::percent::Units).
    fn emit_percent_amount<K>(self, event_key: K, percent: Percent) -> Self
        where
            K: Into<String>,
    {
        self.emit_to_string_value(event_key, percent.units())
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
    fn emit<K, V>(mut self, event_key: K, event_value: V) -> Self where K: Into<String>, V: Into<String> {
        self.event = self.event.add_attribute(event_key, event_value);

        self
    }
}
