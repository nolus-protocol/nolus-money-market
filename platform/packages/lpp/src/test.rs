#![cfg(feature = "testing")]

use currency::NlsPlatform;
use finance::coin::Coin;
use platform::{
    batch::{Batch, Emit, Emitter},
    message::Response as MessageResponse,
};
use sdk::cosmwasm_std::{Addr, StdError};

use crate::{
    error::{Error, Result},
    CoinStable, Lpp,
};

pub struct DummyLpp {
    balance: Option<CoinStable>,
    expected_reward: Option<Coin<NlsPlatform>>,
    failing_reward: bool,
}

impl DummyLpp {
    pub fn with_balance(balance: CoinStable, reward: Coin<NlsPlatform>) -> Self {
        Self {
            balance: Some(balance),
            expected_reward: Some(reward),
            failing_reward: false,
        }
    }

    pub fn failing_balance() -> Self {
        Self {
            balance: None,
            expected_reward: None,
            failing_reward: true,
        }
    }

    pub fn failing_reward(balance: CoinStable, reward: Coin<NlsPlatform>) -> Self {
        Self {
            balance: Some(balance),
            expected_reward: Some(reward),
            failing_reward: true,
        }
    }
}
impl Lpp for DummyLpp {
    fn balance(&self, _oracle: Addr) -> Result<CoinStable> {
        self.balance.ok_or_else(|| {
            Error::Std(StdError::GenericErr {
                msg: "Test failing Lpp::balance()".into(),
            })
        })
    }

    #[allow(clippy::unwrap_in_result)]
    fn ditribute_rewards(self, reward: Coin<NlsPlatform>) -> Result<MessageResponse> {
        assert_eq!(self.expected_reward, Some(reward));

        if self.failing_reward {
            return Err(Error::Std(StdError::generic_err(
                "DummyLpp::distribute_rewards error",
            )));
        }

        let mut msgs = Batch::default();

        #[allow(clippy::unwrap_used)]
        msgs.schedule_execute_wasm_no_reply(Addr::unchecked("Dummy_Lpp"), "message", Some(reward))
            .unwrap();

        let events = Emitter::of_type("eventX").emit_coin("reward", reward);

        Ok(MessageResponse::messages_with_events(msgs, events))
    }
}
