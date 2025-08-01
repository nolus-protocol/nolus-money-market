use std::ops::Add;

use serde::{Deserialize, Serialize};

use currency::platform::Nls;
use finance::{
    coin::Coin,
    price::{self, Price},
    zero::Zero,
};
use lpp_platform::NLpn;
use sdk::{cosmwasm_std::Storage, cw_storage_plus::Item};

use crate::contract::Result;

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Default)]
pub struct TotalRewards();

// TODO avoid potential pragramming mistakes, e.g. use stale index values if load twice, save one,
// and use the other, especially to mutate and override the previous change!
// A potential solution: use Rust ownership model, for example,
// add a read reference to this struct instance as a memver variable of `Index`
impl TotalRewards {
    const REWARDS: Item<Index> = Item::new("total_rewards");

    pub fn load_or_default(store: &dyn Storage) -> Result<Index> {
        Self::REWARDS
            .may_load(store)
            .map_err(Into::into)
            .map(Option::unwrap_or_default)
    }

    pub fn save(rewards: &Index, store: &mut dyn Storage) -> Result<()> {
        Self::REWARDS.save(store, rewards).map_err(Into::into)
    }
}

/// The amount of Nls rewards per receipt
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct Index {
    reward_per_token: Option<Price<NLpn, Nls>>,
}

pub mod migrate_from_0_8_12 {
    use currency::platform::Nls;
    use serde::{Deserialize, Serialize, Serializer};

    use finance::{coin::Coin, price::Price};
    use lpp_platform::NLpn;
    use sdk::{cosmwasm_std::Storage, cw_storage_plus::Item};

    use crate::{
        contract::Result as ContractResult,
        state::{TotalRewards, rewards::Index},
    };

    /// Migrate `DepositsGlobals` into `Index` and return the `balance_nlpn` to be migrated to Total::receipts
    pub fn migrate(store: &mut dyn Storage) -> ContractResult<Coin<NLpn>> {
        #[derive(Deserialize)]
        struct DepositsGlobals {
            balance_nlpn: Coin<NLpn>,

            // Rewards
            reward_per_token: Option<Price<NLpn, Nls>>,
        }
        impl Serialize for DepositsGlobals {
            fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                unimplemented!("satisfies Item::load trait bound")
            }
        }

        let old_key = Item::new("deposits_globals");
        old_key
            .load(store)
            .inspect(|_| old_key.remove(store))
            .map_err(Into::into)
            .and_then(|old_deposits: DepositsGlobals| {
                TotalRewards::save(
                    &Index {
                        reward_per_token: old_deposits.reward_per_token,
                    },
                    store,
                )
                .map(|()| old_deposits.balance_nlpn)
            })
    }
}

impl Index {
    fn new(reward_per_token: Price<NLpn, Nls>) -> Self {
        Self {
            reward_per_token: Some(reward_per_token),
        }
    }
    ///  Calculate rewards
    pub fn rewards(&self, receipts: Coin<NLpn>) -> Coin<Nls> {
        self.reward_per_token
            .map(|price| price::total(receipts, price))
            .unwrap_or_default()
    }

    //TODO migrate this to `checked_add() -> Option<Self>` once `Price::checked_add()` gets available
    pub fn add(self, new_rewards: Coin<Nls>, total_receipts: Coin<NLpn>) -> Self {
        debug_assert_ne!(Coin::ZERO, new_rewards);
        debug_assert_ne!(Coin::ZERO, total_receipts);

        let new_rewards = price::total_of(total_receipts).is(new_rewards);
        if let Some(lhs) = self.reward_per_token {
            Self::new(lhs.add(new_rewards))
        } else {
            Self::new(new_rewards)
        }
    }
}
