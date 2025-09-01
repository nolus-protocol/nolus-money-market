use currency::platform::Nls;
use finance::coin::Coin;
use platform::{
    batch::{Batch, Emit, Emitter},
    message::Response as MessageResponse,
};
use sdk::cosmwasm_std::{Addr, StdError};

use crate::{
    CoinStable, Lpp,
    error::{Error, Result},
};

pub struct DummyLpp {
    balance: Option<CoinStable>,
    expected_reward: Option<Coin<Nls>>,
    failing_reward: bool,
}

impl DummyLpp {
    pub fn with_balance(balance: CoinStable, reward: Coin<Nls>) -> Self {
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

    pub fn failing_reward(balance: CoinStable, reward: Coin<Nls>) -> Self {
        Self {
            balance: Some(balance),
            expected_reward: Some(reward),
            failing_reward: true,
        }
    }
}
impl Lpp for DummyLpp {
    fn balance(&self, _oracle: Addr) -> Result<CoinStable> {
        self.balance
            .ok_or_else(|| Error::Std(StdError::msg("Test failing Lpp::balance()").to_string()))
    }

    fn distribute(self, reward: Coin<Nls>) -> Result<MessageResponse> {
        assert_eq!(self.expected_reward, Some(reward));

        if self.failing_reward {
            return Err(Error::Std(
                StdError::msg("DummyLpp::distribute_rewards error").to_string(),
            ));
        }

        let mut msgs = Batch::default();

        msgs.schedule_execute_wasm_no_reply(Addr::unchecked("Dummy_Lpp"), "message", Some(reward))?;

        let event = Emitter::of_type("eventX").emit_coin("reward", reward);

        Ok(MessageResponse::messages_with_event(msgs, event))
    }
}
