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
    msg::LppBalanceResponse,
    CoinUsd, Lpp,
};

pub struct DummyLpp(Option<LppBalanceResponse>);
impl DummyLpp {
    pub fn with_tvl(tvl: CoinUsd) -> Self {
        Self(Some(LppBalanceResponse {
            balance: tvl,
            total_principal_due: Default::default(),
            total_interest_due: Default::default(),
            balance_nlpn: Default::default(),
        }))
    }

    pub fn failing() -> Self {
        Self(None)
    }
}
impl Lpp for DummyLpp {
    fn balance(&self) -> Result<LppBalanceResponse> {
        self.0.clone().ok_or_else(|| {
            Error::Std(StdError::GenericErr {
                msg: "Test failing Lpp::balance()".into(),
            })
        })
    }

    #[allow(clippy::unwrap_in_result)]
    fn ditribute_rewards(self, reward: Coin<NlsPlatform>) -> Result<MessageResponse> {
        let mut msgs = Batch::default();

        #[allow(clippy::unwrap_used)]
        msgs.schedule_execute_wasm_no_reply(Addr::unchecked("Dummy_Lpp"), "message", Some(reward))
            .unwrap();

        let events = Emitter::of_type("eventX").emit_coin("reward", reward);

        Ok(MessageResponse::messages_with_events(msgs, events))
    }
}

impl AsRef<Self> for DummyLpp {
    fn as_ref(&self) -> &Self {
        self
    }
}
