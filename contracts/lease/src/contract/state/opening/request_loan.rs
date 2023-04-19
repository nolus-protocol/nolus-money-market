use cosmwasm_std::{QuerierWrapper, Timestamp};
use serde::{Deserialize, Serialize};

use dex::IcaConnector;
use lpp::stub::lender::LppLenderRef;
use oracle::stub::OracleRef;
use platform::{
    batch::{Batch, Emit, Emitter},
    message::Response as MessageResponse,
    state_machine::Response as StateMachineResponse,
};
use sdk::cosmwasm_std::{Addr, Deps, DepsMut, Env, MessageInfo, Reply};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{DownpaymentCoin, NewLeaseContract},
    contract::{
        cmd::{OpenLoanReq, OpenLoanReqResult, OpenLoanResp},
        state::{dex::State as LeaseDexState, Handler, Response},
        Contract,
    },
    error::{ContractError, ContractResult},
    event::Type,
    reply_id::ReplyId,
};

use super::open_ica::OpenIcaAccount;

#[derive(Serialize, Deserialize)]
pub(crate) struct RequestLoan {
    new_lease: NewLeaseContract,
    downpayment: DownpaymentCoin,
    deps: (LppLenderRef, OracleRef, TimeAlarmsRef),
}

impl RequestLoan {
    pub fn new(
        deps: &mut DepsMut<'_>,
        info: MessageInfo,
        spec: NewLeaseContract,
    ) -> ContractResult<(Batch, Self)> {
        let lpp = LppLenderRef::try_new(
            spec.form.loan.lpp.clone(),
            &deps.querier,
            ReplyId::OpenLoanReq.into(),
        )?;

        let oracle = OracleRef::try_from(spec.form.market_price_oracle.clone(), &deps.querier)
            .expect("Market Price Oracle is not deployed, or wrong address is passed!");

        let timealarms = TimeAlarmsRef::new(spec.form.time_alarms.clone(), &deps.querier)?;

        let OpenLoanReqResult { batch, downpayment } = lpp.clone().execute(
            OpenLoanReq::new(
                &spec.form.liability,
                info.funds,
                spec.form.max_ltv,
                oracle.clone(),
                &deps.querier,
            ),
            &deps.querier,
        )?;
        Ok((batch, {
            Self {
                new_lease: spec,
                downpayment,
                deps: (lpp, oracle, timealarms),
            }
        }))
    }

    fn on_response(self, deps: Deps<'_>, env: Env, msg: Reply) -> ContractResult<Response> {
        let id = ReplyId::try_from(msg.id)
            .map_err(|_| ContractError::InvalidParameters("Invalid reply ID passed!".into()))?;

        match id {
            ReplyId::OpenLoanReq => {
                let loan = self
                    .deps
                    .0
                    .clone()
                    .execute(OpenLoanResp::new(msg), &deps.querier)?;

                let emitter = self.emit_ok(env.contract.address);

                let open_ica = IcaConnector::new(OpenIcaAccount::new(
                    self.new_lease,
                    self.downpayment,
                    loan,
                    self.deps,
                ));
                Ok(StateMachineResponse::from(
                    MessageResponse::messages_with_events(open_ica.enter(), emitter),
                    LeaseDexState::new(open_ica),
                ))
            }
        }
    }

    fn emit_ok(&self, contract: Addr) -> Emitter {
        Emitter::of_type(Type::RequestLoan).emit("id", contract)
    }
}

impl Handler for RequestLoan {
    fn reply(self, deps: &mut DepsMut<'_>, env: Env, msg: Reply) -> ContractResult<Response> {
        self.on_response(deps.as_ref(), env, msg)
    }
}

impl Contract for RequestLoan {
    fn state(
        self,
        _now: Timestamp,
        _querier: &QuerierWrapper<'_>,
    ) -> ContractResult<crate::api::StateResponse> {
        unreachable!()
    }
}
