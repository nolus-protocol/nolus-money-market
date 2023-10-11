use serde::{Deserialize, Serialize};

use lpp::stub::LppRef;
use oracle::stub::OracleRef;
use platform::{
    batch::{Batch, Emit, Emitter},
    message::Response as MessageResponse,
    state_machine::Response as StateMachineResponse,
};
use sdk::cosmwasm_std::{Addr, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, Reply, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{DownpaymentCoin, NewLeaseContract},
    contract::{
        cmd::{OpenLoanReq, OpenLoanReqResult, OpenLoanResp},
        finalize::FinalizerRef,
        state::{Handler, Response},
    },
    error::ContractResult,
    event::Type,
};

use super::buy_asset::DexState;

#[derive(Serialize, Deserialize)]
pub(crate) struct RequestLoan {
    new_lease: NewLeaseContract,
    downpayment: DownpaymentCoin,
    deps: (LppRef, OracleRef, TimeAlarmsRef, FinalizerRef),
}

impl RequestLoan {
    pub fn new(
        deps: &mut DepsMut<'_>,
        info: MessageInfo,
        spec: NewLeaseContract,
    ) -> ContractResult<(Batch, Self)> {
        let lpp = LppRef::try_new(spec.form.loan.lpp.clone(), &deps.querier)?;

        let oracle = OracleRef::try_from(spec.form.market_price_oracle.clone(), &deps.querier)
            .expect("Market Price Oracle is not deployed, or wrong address is passed!");

        let timealarms = TimeAlarmsRef::new(spec.form.time_alarms.clone(), &deps.querier)?;

        let finalizer = FinalizerRef::try_new(spec.finalizer.clone(), &deps.querier)?;

        let OpenLoanReqResult { batch, downpayment } = lpp.clone().execute_lender(
            OpenLoanReq::new(
                spec.form.position_spec.clone(),
                info.funds,
                spec.form.max_ltd,
                oracle.clone(),
                &deps.querier,
            ),
            &deps.querier,
        )?;
        Ok((batch, {
            Self {
                new_lease: spec,
                downpayment,
                deps: (lpp, oracle, timealarms, finalizer),
            }
        }))
    }

    fn on_response(self, deps: Deps<'_>, env: Env, msg: Reply) -> ContractResult<Response> {
        let loan = self
            .deps
            .0
            .clone()
            .execute_lender(OpenLoanResp::new(msg), &deps.querier)?;

        let emitter = self.emit_ok(env.contract.address);

        let open_ica = super::buy_asset::start(
            self.new_lease,
            self.downpayment,
            loan,
            self.deps,
            env.block.time,
        );
        Ok(StateMachineResponse::from(
            MessageResponse::messages_with_events(open_ica.enter(), emitter),
            Into::<DexState>::into(open_ica),
        ))
    }

    fn emit_ok(&self, contract: Addr) -> Emitter {
        Emitter::of_type(Type::RequestLoan).emit("id", contract)
    }
}

impl Handler for RequestLoan {
    fn state(
        self,
        _now: Timestamp,
        _querier: &QuerierWrapper<'_>,
    ) -> ContractResult<crate::api::StateResponse> {
        unreachable!()
    }

    fn reply(self, deps: &mut DepsMut<'_>, env: Env, msg: Reply) -> ContractResult<Response> {
        self.on_response(deps.as_ref(), env, msg)
    }
}
