use currency::NlsPlatform;
use finance::coin::Coin;
use platform::{
    batch::{Batch, Emit, Emitter},
    message::Response as MessageResponse,
};
use sdk::cosmwasm_std::{Addr, Env, QuerierWrapper};

use crate::{
    error::Result,
    msg::{ExecuteMsg, QueryMsg},
    CoinStable, Lpp,
};

pub struct Stub<'a> {
    lpp: Addr,
    oracle: Addr,
    querier: QuerierWrapper<'a>,
    env: &'a Env,
}

impl<'a> Stub<'a> {
    pub(crate) fn new(lpp: Addr, oracle: Addr, querier: QuerierWrapper<'a>, env: &'a Env) -> Self {
        Self {
            lpp,
            oracle,
            querier,
            env,
        }
    }
}

impl<'a> Lpp for Stub<'a> {
    fn balance(&self) -> Result<CoinStable> {
        self.querier
            .query_wasm_smart(
                &self.lpp,
                &(QueryMsg::StableBalance {
                    oracle_addr: self.oracle.clone(),
                }),
            )
            .map_err(Into::into)
    }

    fn ditribute_rewards(self, reward: Coin<NlsPlatform>) -> Result<MessageResponse> {
        if reward.is_zero() {
            return Ok(Default::default());
        }

        let mut msgs = Batch::default();
        msgs.schedule_execute_wasm_no_reply(
            self.lpp.clone(),
            &ExecuteMsg::DistributeRewards {},
            Some(reward),
        )
        .map(|()| {
            Emitter::of_type("tr-rewards")
                .emit_tx_info(self.env)
                .emit_to_string_value("to", self.lpp)
                .emit_coin("rewards", reward)
        })
        .map(|events| MessageResponse::messages_with_events(msgs, events))
        .map_err(Into::into)
    }
}

impl<'a> AsRef<Stub<'a>> for Stub<'a> {
    fn as_ref(&self) -> &Self {
        self
    }
}

#[cfg(test)]
mod test {
    use platform::message::Response as MessageResponse;
    use sdk::cosmwasm_std::{
        testing::{self, MockQuerier},
        Addr, QuerierWrapper,
    };

    use crate::Lpp;

    use super::Stub;

    #[test]
    fn ditribute_no_reward() {
        let mock_querier = MockQuerier::default();
        let env = testing::mock_env();
        let querier = QuerierWrapper::new(&mock_querier);
        let lpp_addr = Addr::unchecked("LPP");
        let oracle_addr = Addr::unchecked("ORACLE");
        let stub = Stub::new(lpp_addr, oracle_addr, querier, &env);
        assert_eq!(
            Ok(MessageResponse::default()),
            stub.ditribute_rewards(0.into())
        );
    }
}
