use std::ops::{Deref, DerefMut};

use currency::{CurrencyDTO, Group};
use sdk::cosmwasm_std::Storage;

use crate::{
    alarms::prefix::Prefix,
    feed::{
        observations::{ObservationsReadRepo, ObservationsRepo},
        Observations, ObservationsRead,
    },
};

use super::observations::Deque;

pub struct Repo<'storage, S>
where
    S: Deref<Target = dyn Storage + 'storage>,
{
    root_ns: &'static str,
    storage: S,
}

impl<'storage, S> Repo<'storage, S>
where
    S: Deref<Target = dyn Storage + 'storage>,
{
    pub fn new(root_ns: &'static str, storage: S) -> Self {
        Self { root_ns, storage }
    }

    fn storage_ns<G>(
        &self,
        c: currency::CurrencyDTO<G>,
        quote_c: currency::CurrencyDTO<G>,
    ) -> String
    where
        G: Group,
    {
        format!("{}_{}_{}", self.root_ns, c.first_key(), quote_c.first_key())
    }
}

impl<'storage, S> ObservationsReadRepo for Repo<'storage, S>
where
    S: Deref<Target = dyn Storage + 'storage>,
{
    fn observations_read<C, QuoteC, G>(
        &self,
        c: currency::CurrencyDTO<G>,
        quote_c: currency::CurrencyDTO<G>,
    ) -> impl ObservationsRead<C, QuoteC>
    where
        C: 'static,
        QuoteC: 'static,
        G: Group,
    {
        Deque::new(self.storage.deref(), self.storage_ns(c, quote_c))
    }
}

impl<'storage, S> ObservationsRepo for Repo<'storage, S>
where
    S: Deref<Target = dyn Storage + 'storage> + DerefMut,
{
    fn observations<C, QuoteC, G>(
        &mut self,
        c: CurrencyDTO<G>,
        quote_c: CurrencyDTO<G>,
    ) -> impl Observations<C, QuoteC>
    where
        C: 'static,
        QuoteC: 'static,
        G: Group,
    {
        let namespace = self.storage_ns(c, quote_c);
        Deque::new(self.storage.deref_mut(), namespace)
    }
}
