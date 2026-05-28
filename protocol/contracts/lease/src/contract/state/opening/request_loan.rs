use finance::duration::Duration;
use serde::{Deserialize, Serialize};

use finance::instant::Instant;
use platform::{
    batch::{Batch, Emit, Emitter},
    contract,
    message::Response as MessageResponse,
    state_machine::Response as StateMachineResponse,
};
use sdk::cosmwasm_std::{Addr, Env, MessageInfo, QuerierWrapper, Reply};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{DownpaymentCoin, open::NewLeaseContract, query::StateResponse as QueryStateResponse},
    contract::{
        cmd::{OpenLoanReq, OpenLoanReqResult, OpenLoanResp},
        finalize::LeasesRef,
        state::{Handler, Response, State},
    },
    error::{ContractError, ContractResult},
    event::Type,
    finance::{LppRef, OracleRef},
};

use super::open_lease::OpenLease;
use cw_time::IntoInstant;

#[derive(Serialize, Deserialize)]
pub(crate) struct RequestLoan {
    new_lease: NewLeaseContract,
    downpayment: DownpaymentCoin,
    deps: (LppRef, OracleRef, TimeAlarmsRef, LeasesRef),
}

impl RequestLoan {
    pub fn new(
        querier: QuerierWrapper<'_>,
        info: MessageInfo,
        spec: NewLeaseContract,
    ) -> ContractResult<(Batch, Self)> {
        let addr_validator = contract::validator(querier);
        let lpp = LppRef::try_new(spec.form.loan.lpp.clone(), querier)
            .map_err(ContractError::LppStubError)?;

        let oracle = OracleRef::try_from_base(spec.form.market_price_oracle.clone(), querier)
            .map_err(ContractError::CrateOracleRef)?;

        let timealarms = TimeAlarmsRef::new(spec.form.time_alarms.clone(), &addr_validator)?;

        let finalizer = LeasesRef::try_new(spec.finalizer.clone(), &addr_validator)?;

        let OpenLoanReqResult { batch, downpayment } = lpp.clone().execute_lender(
            OpenLoanReq::new(
                spec.form.position_spec,
                info.funds,
                spec.form.max_ltd,
                oracle.clone(),
                querier,
            ),
            querier,
        )?;
        Ok((batch, {
            Self {
                new_lease: spec,
                downpayment,
                deps: (lpp, oracle, timealarms, finalizer),
            }
        }))
    }

    fn on_response(
        self,
        querier: QuerierWrapper<'_>,
        env: Env,
        msg: Reply,
    ) -> ContractResult<Response> {
        let loan = self
            .deps
            .0
            .clone()
            .execute_lender(OpenLoanResp::new(msg), querier)?;

        let emitter = self.emit_ok(env.contract.address);

        let open_lease = OpenLease::new(
            self.new_lease,
            self.downpayment,
            loan,
            self.deps,
            env.block.time.into_instant(),
        );
        open_lease.enter().map(|batch| {
            StateMachineResponse::from(
                MessageResponse::messages_with_event(batch, emitter),
                State::from(open_lease),
            )
        })
    }

    fn emit_ok(&self, contract: Addr) -> Emitter {
        Emitter::of_type(Type::RequestLoan).emit("id", contract)
    }
}

impl Handler for RequestLoan {
    fn state(
        self,
        _now: Instant,
        _due_projection: Duration,
        _querier: QuerierWrapper<'_>,
    ) -> ContractResult<QueryStateResponse> {
        unreachable!()
    }

    fn reply(self, querier: QuerierWrapper<'_>, env: Env, msg: Reply) -> ContractResult<Response> {
        self.on_response(querier, env, msg)
    }
}
