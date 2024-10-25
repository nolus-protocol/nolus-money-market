use std::ops::{Deref, DerefMut};

use sdk::{
    cosmwasm_std::{Storage, Timestamp},
    cw_storage_plus::{Deque as CwDeque, Namespace},
};

use crate::{
    error::PriceFeedsError,
    feed::{self, Observation, Observations, ObservationsRead},
};

pub(super) struct Deque<'storage, C, QuoteC, S>
where
    C: 'static,
    QuoteC: 'static,
    S: Deref<Target = dyn Storage + 'storage>,
{
    storage: CwDeque<Observation<C, QuoteC>>,
    store: S,
}

impl<'storage, C, QuoteC, S> Deque<'storage, C, QuoteC, S>
where
    C: 'static,
    QuoteC: 'static,
    S: Deref<Target = dyn Storage + 'storage>,
{
    pub(super) fn new<NameSpace>(store: S, namespace: NameSpace) -> Self
    where
        NameSpace: Into<Namespace>,
    {
        Self {
            storage: CwDeque::new_dyn(namespace),
            store,
        }
    }
}

impl<'storage, C, QuoteC, S> ObservationsRead<C, QuoteC> for Deque<'storage, C, QuoteC, S>
where
    C: 'static,
    QuoteC: 'static,
    S: Deref<Target = dyn Storage + 'storage>,
{
    fn len(&self) -> usize {
        self.storage
            .len(self.store.deref())
            .expect("u32 to fit in usize") as usize
    }

    fn as_iter(
        &self,
    ) -> Result<
        impl Iterator<Item = Result<Observation<C, QuoteC>, PriceFeedsError>>,
        PriceFeedsError,
    > {
        self.storage
            .iter(self.store.deref())
            .map(|iter| iter.map(|item| item.map_err(PriceFeedsError::FeedRead)))
            .map_err(PriceFeedsError::FeedRead)
    }
}

impl<'storage, C, QuoteC, S> Observations<C, QuoteC> for Deque<'storage, C, QuoteC, S>
where
    C: 'static,
    QuoteC: 'static,
    S: Deref<Target = dyn Storage + 'storage> + DerefMut,
{
    fn retain(&mut self, valid_since: Timestamp) -> Result<(), PriceFeedsError> {
        let valid_fn = feed::valid_since(valid_since);
        loop {
            match self
                .storage
                .pop_front(self.store.deref_mut())
                .map_err(PriceFeedsError::FeedRemove)
                .and_then(|may_item| {
                    if let Some(item) = may_item {
                        if valid_fn(&item) {
                            self.storage
                                .push_back(self.store.deref_mut(), &item)
                                .map_err(PriceFeedsError::FeedPush)
                                .map(|()| false)
                        } else {
                            Ok(true)
                        }
                    } else {
                        Ok(false)
                    }
                }) {
                Ok(true) => continue,
                Ok(false) => break Ok(()),
                Err(err) => break Err(err),
            }
        }
    }

    fn register(&mut self, observation: Observation<C, QuoteC>) -> Result<(), PriceFeedsError> {
        self.storage
            .push_back(self.store.deref_mut(), &observation)
            .map_err(PriceFeedsError::FeedPush)
    }
}
