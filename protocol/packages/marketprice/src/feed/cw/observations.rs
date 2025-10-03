use std::ops::{Deref, DerefMut};

use sdk::{
    cosmwasm_std::{Storage, Timestamp},
    cw_storage_plus::{Deque as CwDeque, Namespace},
};

use crate::{
    error::PriceFeedsError,
    feed::{Observation, Observations, ObservationsRead},
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

impl<'storage, C, QuoteC, S> ObservationsRead for Deque<'storage, C, QuoteC, S>
where
    C: 'static,
    QuoteC: 'static,
    S: Deref<Target = dyn Storage + 'storage>,
{
    type C = C;

    type QuoteC = QuoteC;

    fn len(&self) -> usize {
        self.storage
            .len(self.store.deref())
            .expect("u32 to fit in usize") as usize
    }

    fn as_iter(
        &self,
    ) -> Result<
        impl Iterator<Item = Result<Observation<C, QuoteC>, PriceFeedsError>> + '_,
        PriceFeedsError,
    > {
        self.storage
            .iter(self.store.deref())
            .map(|iter| iter.map(|item| item.map_err(PriceFeedsError::FeedRead)))
            .map_err(PriceFeedsError::FeedRead)
    }
}

impl<'storage, C, QuoteC, S> Observations for Deque<'storage, C, QuoteC, S>
where
    C: 'static,
    QuoteC: 'static,
    S: Deref<Target = dyn Storage + 'storage> + DerefMut,
{
    fn retain(&mut self, valid_since: &Timestamp) -> Result<(), PriceFeedsError> {
        loop {
            match self
                .storage
                .pop_front(self.store.deref_mut())
                .map_err(PriceFeedsError::FeedRemove)
                .and_then(|may_item| {
                    if let Some(item) = may_item {
                        if item.valid_since(valid_since) {
                            self.storage
                                .push_front(self.store.deref_mut(), &item)
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

#[cfg(test)]
mod test {
    use std::fmt::Debug;

    use currency::test::{SuperGroupTestC4, SuperGroupTestC5};
    use finance::{
        coin::{Amount, Coin},
        duration::Duration,
        price::{self, Price},
    };
    use sdk::cosmwasm_std::{Addr, Storage, Timestamp, testing::MockStorage};

    use crate::feed::observations::{Observations, ObservationsRead};

    use super::{Deque, Observation};

    const VALIDITY_PERIOD: Duration = Duration::from_secs(60);
    const BLOCK_TIME: Timestamp = Timestamp::from_seconds(150);
    const FEEDER: &str = "feeder1";

    type C1 = SuperGroupTestC4;
    type C2 = SuperGroupTestC5;

    #[test]
    fn retain() {
        let mut store = MockStorage::new();
        let mut deque = Deque::new(&mut store as &mut dyn Storage, "");

        let feed1_time = BLOCK_TIME - VALIDITY_PERIOD;
        let feed1_price = price(19, 5100);
        let observation1 = Observation::new(Addr::unchecked(FEEDER), feed1_time, feed1_price);
        deque.register(observation1.clone()).unwrap();
        assert_eq!(1, deque.len());
        assert_eq!(Some(Ok(observation1)), deque.as_iter().unwrap().next());

        deque
            .retain(&(feed1_time - Duration::from_nanos(1)))
            .unwrap();
        assert_eq!(1, deque.len());

        deque.retain(&feed1_time).unwrap();
        assert_eq!(0, deque.len());
    }

    #[test]
    fn retain_order() {
        let mut store = MockStorage::new();
        let mut deque = Deque::new(&mut store as &mut dyn Storage, "");

        let feed1_time = BLOCK_TIME - VALIDITY_PERIOD;
        let feed1_price = price(19, 5100);
        let observation1 = Observation::new(Addr::unchecked(FEEDER), feed1_time, feed1_price);
        deque.register(observation1.clone()).unwrap();

        let feed2_time = feed1_time + VALIDITY_PERIOD;
        let feed2_price = price(20, 5100);
        let observation2 = Observation::new(Addr::unchecked(FEEDER), feed2_time, feed2_price);
        deque.register(observation2.clone()).unwrap();

        assert_eq!(2, deque.len());
        assert_elements(&deque, observation1.clone(), observation2.clone());

        deque
            .retain(&(feed1_time - Duration::from_nanos(1)))
            .unwrap();
        assert_eq!(2, deque.len());
        assert_elements(&deque, observation1.clone(), observation2.clone());

        deque.retain(&feed1_time).unwrap();
        assert_eq!(1, deque.len());

        deque.retain(&feed2_time).unwrap();
        assert_eq!(0, deque.len());
    }

    fn assert_elements<C, QuoteC, ObsRead>(
        deque: &ObsRead,
        observation1: Observation<C, QuoteC>,
        observation2: Observation<C, QuoteC>,
    ) where
        ObsRead: ObservationsRead<C = C, QuoteC = QuoteC>,
        C: Debug + PartialEq,
        QuoteC: Debug + PartialEq,
    {
        let mut it = deque.as_iter().unwrap();
        assert_eq!(Some(Ok(observation1)), it.next());
        assert_eq!(Some(Ok(observation2)), it.next());
    }

    fn price(c: Amount, q: Amount) -> Price<C1, C2> {
        price::total_of(Coin::new(c)).is(Coin::new(q))
    }
}
