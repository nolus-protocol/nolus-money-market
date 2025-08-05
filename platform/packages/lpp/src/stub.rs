use currency::platform::{Nls, PlatformGroup};
use finance::coin::{Coin, CoinDTO};
use platform::{
    batch::{Batch, Emit, Emitter},
    message::Response as MessageResponse,
};
use sdk::cosmwasm_std::{Addr, Env, QuerierWrapper};

use crate::{
    CoinStable, Lpp,
    error::Result,
    msg::{ExecuteMsg, QueryMsg},
};

pub struct Stub<'querier, 'env> {
    lpp: Addr,
    querier: QuerierWrapper<'querier>,
    env: &'env Env,
}

impl<'querier, 'env> Stub<'querier, 'env> {
    pub(crate) fn new(lpp: Addr, querier: QuerierWrapper<'querier>, env: &'env Env) -> Self {
        Self { lpp, querier, env }
    }
}

impl Lpp for Stub<'_, '_> {
    fn balance(&self, oracle: Addr) -> Result<CoinStable> {
        self.querier
            .query_wasm_smart::<CoinDTO<PlatformGroup>>(
                &self.lpp,
                &(QueryMsg::StableBalance {
                    oracle_addr: oracle,
                }),
            )
            .map_err(Into::into)
            .map(|dto| {
                dto.try_into()
                    .unwrap_or_else(|_| unreachable!("Each stable is member of the plaform group!"))
            })
    }

    fn distribute(self, reward: Coin<Nls>) -> Result<MessageResponse> {
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
        .map(|event| MessageResponse::messages_with_event(msgs, event))
        .map_err(Into::into)
    }
}

#[cfg(test)]
mod test {
    use platform::message::Response as MessageResponse;
    use sdk::cosmwasm_std::{
        Addr, QuerierWrapper,
        testing::{self, MockQuerier},
    };

    use crate::Lpp;

    use super::Stub;

    #[test]
    fn ditribute_no_reward() {
        let mock_querier = MockQuerier::default();
        let env = testing::mock_env();
        let querier = QuerierWrapper::new(&mock_querier);
        let lpp_addr = Addr::unchecked("LPP");
        let stub = Stub::new(lpp_addr, querier, &env);
        assert_eq!(Ok(MessageResponse::default()), stub.distribute(0.into()));
    }
}
