use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use currency::{CurrencyDTO, Group};
use sdk::cosmwasm_std::Storage;

use crate::{
    alarms::prefix::Prefix,
    feed::observations::{ObservationsReadRepo, ObservationsRepo},
};

use super::observations::Deque;

pub struct Repo<'storage, S, G>
where
    S: Deref<Target = dyn Storage + 'storage>,
{
    root_ns: &'static str,
    storage: S,
    _group: PhantomData<G>,
}

impl<'storage, S, G> Repo<'storage, S, G>
where
    S: Deref<Target = dyn Storage + 'storage>,
    G: Group,
{
    pub fn new(root_ns: &'static str, storage: S) -> Self {
        Self {
            root_ns,
            storage,
            _group: PhantomData,
        }
    }

    fn storage_ns(&self, c: &CurrencyDTO<G>, quote_c: &CurrencyDTO<G>) -> String {
        format!("{}_{}_{}", self.root_ns, c.first_key(), quote_c.first_key())
    }
}

impl<'storage, S, G> ObservationsReadRepo for Repo<'storage, S, G>
where
    S: Deref<Target = dyn Storage + 'storage>,
    G: Group,
{
    type Group = G;

    type ObservationsRead<'r, C, QuoteC>
        = Deque<'r, C, QuoteC, &'r (dyn Storage + 'r)>
    where
        Self: 'r,
        C: 'static,
        QuoteC: 'static;

    // TODO[feature(precise_capturing_in_traits)]
    //  Refactor to use precise capturing when stabilized.
    fn observations_read<'self_, C, QuoteC>(
        &'self_ self,
        c: &CurrencyDTO<Self::Group>,
        quote_c: &CurrencyDTO<Self::Group>,
    ) -> Self::ObservationsRead<'self_, C, QuoteC>
    where
        C: 'static,
        QuoteC: 'static,
    {
        Deque::new(self.storage.deref(), self.storage_ns(c, quote_c))
    }
}

impl<'storage, S, G> ObservationsRepo for Repo<'storage, S, G>
where
    S: Deref<Target = dyn Storage + 'storage> + DerefMut,
    G: Group,
{
    type Observations<'r, C, QuoteC>
        = Deque<'r, C, QuoteC, &'r mut (dyn Storage + 'r)>
    where
        Self: 'r,
        C: 'static,
        QuoteC: 'static;

    // TODO[feature(precise_capturing_in_traits)]
    //  Refactor to use precise capturing when stabilized.
    fn observations<'self_, C, QuoteC>(
        &'self_ mut self,
        c: &CurrencyDTO<Self::Group>,
        quote_c: &CurrencyDTO<Self::Group>,
    ) -> Self::Observations<'self_, C, QuoteC>
    where
        C: 'static,
        QuoteC: 'static,
    {
        let namespace = self.storage_ns(c, quote_c);
        Deque::new(self.storage.deref_mut(), namespace)
    }
}
