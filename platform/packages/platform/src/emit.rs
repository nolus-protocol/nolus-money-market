use currency::{CurrencyDTO, CurrencyDef, Group};
use finance::{
    coin::{Amount, Coin, CoinDTO},
    percent::Percent,
};
use sdk::cosmwasm_std::{Env, Event, Timestamp};

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
        C: CurrencyDef,
    {
        self.emit(event_key, C::dto().definition().ticker)
    }

    /// Specialization of [`emit`](Self::emit) for [`Currency`]'s symbol.
    fn emit_currency_dto<K, G>(self, event_key: K, currency: &CurrencyDTO<G>) -> Self
    where
        K: Into<String>,
        G: Group,
    {
        self.emit(event_key, currency.to_string())
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
        C: CurrencyDef,
    {
        self.emit_coin_dto(event_key, &CoinDTO::<C::Group>::from(coin))
    }

    fn emit_coin_dto<K, G>(self, event_key: K, coin: &CoinDTO<G>) -> Self
    where
        K: Into<String>,
        G: Group,
    {
        emit_coinable(self, event_key, coin.amount(), &coin.currency())
    }

    fn emit_tx_info(self, env: &Env) -> Self {
        self.emit_to_string_value("height", env.block.height)
            .emit_timestamp("at", &env.block.time)
            .emit_to_string_value(
                "idx",
                // TODO use `.expect(...)` when layer 1 upgrades to `wasmd` v0.29
                env.transaction
                    .as_ref()
                    .map(|transaction| transaction.index)
                    .unwrap_or_default(),
            )
    }
}

#[cfg_attr(any(test, feature = "testing"), derive(Debug, PartialEq))]
pub struct Emitter {
    event: Event,
}

impl Emitter {
    pub fn of_type<T>(event_type: T) -> Self
    where
        T: Into<String>,
    {
        Self {
            event: Event::new(event_type.into()),
        }
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

impl From<Emitter> for Event {
    fn from(value: Emitter) -> Self {
        value.event
    }
}

fn emit_coinable<E, K, G>(emitter: E, event_key: K, amount: Amount, currency: &CurrencyDTO<G>) -> E
where
    E: Emit,
    K: Into<String>,
    G: Group,
{
    let key = event_key.into();
    let amount_key = key.clone() + "-amount";
    let symbol_key = key + "-symbol";

    emitter
        .emit_coin_amount(amount_key, amount)
        .emit_currency_dto(symbol_key, currency)
}
