use cosmwasm_std::{QuerierWrapper, Timestamp};
use serde::{Deserialize, Serialize};

use lpp::stub::lender::LppLenderRef;
use oracle::stub::OracleRef;
use platform::batch::{Batch, Emit, Emitter};
use sdk::cosmwasm_std::{Addr, Deps, DepsMut, Env, MessageInfo, Reply};

use crate::{
    api::{DownpaymentCoin, NewLeaseContract},
    contract::{
        cmd::{OpenLoanReq, OpenLoanReqResult, OpenLoanResp},
        state::{ica_connector::IcaConnector, Controller, Response},
        Contract,
    },
    error::{ContractError, ContractResult},
    event::Type,
    reply_id::ReplyId,
};

use super::open_ica::OpenIcaAccount;

#[derive(Serialize, Deserialize)]
pub struct RequestLoan {
    new_lease: NewLeaseContract,
    downpayment: DownpaymentCoin,
    deps: (LppLenderRef, OracleRef),
}

impl RequestLoan {
    pub fn new(
        deps: &mut DepsMut<'_>,
        info: MessageInfo,
        new_lease: NewLeaseContract,
    ) -> ContractResult<(Batch, Self)> {
        let lpp = LppLenderRef::try_new(
            new_lease.form.loan.lpp.clone(),
            &deps.querier,
            ReplyId::OpenLoanReq.into(),
        )?;

        let oracle = OracleRef::try_from(new_lease.form.market_price_oracle.clone(), &deps.querier)
            .expect("Market Price Oracle is not deployed, or wrong address is passed!");

        let OpenLoanReqResult { batch, downpayment } = lpp.clone().execute(
            OpenLoanReq::new(
                &new_lease.form.liability,
                info.funds,
                oracle.clone(),
                &deps.querier,
            ),
            &deps.querier,
        )?;
        Ok((
            batch,
            RequestLoan {
                new_lease,
                downpayment,
                deps: (lpp, oracle),
            },
        ))
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
                Ok(Response::from(
                    open_ica.enter().into_response(emitter),
                    open_ica,
                ))
            }
        }
    }

    fn emit_ok(&self, contract: Addr) -> Emitter {
        Emitter::of_type(Type::RequestLoan).emit("id", contract)
    }
}

impl Controller for RequestLoan {
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
