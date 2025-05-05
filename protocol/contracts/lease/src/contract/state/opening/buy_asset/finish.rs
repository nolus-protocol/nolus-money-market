use std::marker::PhantomData;

use currency::{CurrencyDef, Group, MemberOf};
use profit::stub::ProfitRef;

use dex::{AnomalyTreatment, SwapOutputTask, SwapTask};
use finance::coin::Coin;
use platform::{
    message::Response as MessageResponse, state_machine::Response as StateMachineResponse,
};
use sdk::cosmwasm_std::{Env, QuerierWrapper};

use crate::{
    contract::{
        Lease,
        cmd::{CloseStatusDTO, LeaseFactory, OpenLeaseResult},
        state::opened::{active::Active, close::liquidation},
    },
    finance::ReserveRef,
    lease::with_lease_deps,
    position::Position,
};

use super::BuyAsset;

pub struct BuyAssetFinish<SwapTask, OutC> {
    swap_task: SwapTask,
    _out_c: PhantomData<OutC>,
}

impl<SwapTask, OutC> BuyAssetFinish<SwapTask, OutC> {
    pub fn from(swap_task: SwapTask) -> Self {
        Self {
            swap_task,
            _out_c: PhantomData,
        }
    }
}

impl<OutC> SwapOutputTask<BuyAsset> for BuyAssetFinish<BuyAsset, OutC>
where
    OutC: CurrencyDef,
    OutC::Group: MemberOf<<BuyAsset as SwapTask>::OutG>
        + MemberOf<<<BuyAsset as SwapTask>::InG as Group>::TopG>,
{
    type OutC = OutC;

    fn as_spec(&self) -> &BuyAsset {
        &self.swap_task
    }

    fn into_spec(self) -> BuyAsset {
        self.swap_task
    }

    fn on_anomaly(self) -> AnomalyTreatment<BuyAsset>
    where
        Self: Sized,
    {
        AnomalyTreatment::Retry(self.into_spec())
    }

    fn finish(
        self,
        amount_out: Coin<Self::OutC>,
        env: &Env,
        querier: QuerierWrapper<'_>,
    ) -> <BuyAsset as SwapTask>::Result {
        let spec = self.into_spec();
        debug_assert!(
            spec.form
                .currency
                .into_super_group::<<BuyAsset as SwapTask>::OutG>()
                .of_currency(Self::OutC::dto())
                .is_ok()
        );
        debug_assert!(!amount_out.is_zero());

        let position = spec
            .form
            .position_spec
            .try_into()
            .map(|spec| Position::new(amount_out, spec))?;
        let lease_addr = spec.dex_account.owner().clone();
        let cmd = {
            let profit = ProfitRef::new(spec.form.loan.profit.clone(), &querier)?;
            let reserve = ReserveRef::try_new(spec.form.reserve.clone(), &querier)?;

            LeaseFactory::new(
                spec.form,
                lease_addr.clone(),
                profit,
                reserve,
                (spec.deps.2, spec.deps.1.clone()),
                spec.start_opening_at,
                &env.block.time,
            )
        };
        let OpenLeaseResult { lease, status } = with_lease_deps::execute_resolved_position(
            cmd,
            lease_addr,
            position,
            spec.deps.0,
            spec.deps.1,
            querier,
        )?;

        let lease = Lease::new(lease, spec.dex_account, spec.deps.3);
        let active = Active::new(lease);
        let emitter = active.emit_opened(env, spec.downpayment, spec.loan);

        match status {
            CloseStatusDTO::Paid => {
                unimplemented!("a freshly open lease should have some due amount")
            }
            CloseStatusDTO::None {
                current_liability: _, // TODO shouldn't we add warning zone events?
                alarms,
            } => Ok(StateMachineResponse::from(
                MessageResponse::messages_with_events(alarms, emitter),
                active,
            )),
            CloseStatusDTO::NeedLiquidation(liquidation) => {
                liquidation::start(active.into(), liquidation, emitter.into(), env, querier)
            }
            CloseStatusDTO::CloseAsked(_) => unimplemented!("no triggers have been set"),
        }
    }
}
