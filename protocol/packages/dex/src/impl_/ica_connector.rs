use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    marker::PhantomData,
};

use finance::duration::Duration;
use serde::{Deserialize, Serialize};

use finance::instant::Instant;
use platform::{
    batch::{Batch, Emit, Emitter},
    ica::HostAccount,
    message,
};
use sdk::cosmwasm_std::{Addr, Env, QuerierWrapper};

use crate::{
    Account, Connectable, ContinueResult, Contract, Enterable, Handler, IcaConnectee, Response,
    TimeAlarm, error::Result,
};

#[cfg(feature = "migration")]
use super::migration::MigrateSpec;
use cw_time::IntoInstant;

#[derive(Serialize, Deserialize)]
#[serde(bound(
    serialize = "Connectee: Serialize",
    deserialize = "Connectee: Deserialize<'de>",
))]
pub struct IcaConnector<Connectee, SwapResult> {
    connectee: Connectee,
    #[serde(skip)]
    _swap_result: PhantomData<SwapResult>,
}

impl<Connectee, SwapResult> IcaConnector<Connectee, SwapResult>
where
    Connectee: IcaConnectee + Connectable,
{
    const STATE_LABEL: &'static str = "register-ica";

    pub fn new(connectee: Connectee) -> Self {
        Self {
            connectee,
            _swap_result: PhantomData,
        }
    }

    pub fn enter(&self) -> Batch {
        Account::register_request(self.connectee.dex())
    }

    fn build_account(&self, counterparty_version: String, env: &Env) -> Result<Account> {
        let contract = env.contract.address.clone();
        Account::from_register_response(
            &counterparty_version,
            contract,
            self.connectee.dex().clone(),
        )
    }

    fn emit_ok(contract: Addr, ica_host: HostAccount) -> Emitter {
        Emitter::of_type(Self::STATE_LABEL)
            .emit("id", contract)
            .emit("ica_host", ica_host)
    }
}

#[cfg(feature = "migration")]
impl<SwapTask, SwapTaskNew, SEnumNew, Connectee, SwapResult>
    MigrateSpec<SwapTask, SwapTaskNew, SEnumNew> for IcaConnector<Connectee, SwapResult>
where
    Connectee: MigrateSpec<SwapTask, SwapTaskNew, SEnumNew>,
    Connectee::Out: IcaConnectee + Connectable,
{
    type Out = IcaConnector<Connectee::Out, SwapResult>;

    fn migrate_spec<MigrateFn>(self, migrate_fn: MigrateFn) -> Self::Out
    where
        MigrateFn: FnOnce(SwapTask) -> SwapTaskNew,
    {
        Self::Out::new(self.connectee.migrate_spec(migrate_fn))
    }
}

impl<Connectee, SwapResult> Enterable for IcaConnector<Connectee, SwapResult>
where
    Connectee: IcaConnectee + Connectable,
{
    fn enter(&self, _now: Instant, _querier: QuerierWrapper<'_>) -> Result<Batch> {
        Ok(self.enter())
    }
}

impl<Connectee, SwapResult> Handler for IcaConnector<Connectee, SwapResult>
where
    Connectee: IcaConnectee + Connectable + Display,
{
    type Response = Connectee::State;
    type SwapResult = SwapResult;

    fn authz_remote_callback(
        &self,
        querier: QuerierWrapper<'_>,
        info: &sdk::cosmwasm_std::MessageInfo,
    ) -> crate::error::Result<()> {
        self.connectee.authz_remote_callback(querier, info)
    }

    fn on_open_ica(
        self,
        counterparty_version: String,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> ContinueResult<Self> {
        let ica = self.build_account(counterparty_version, &env)?;
        let event = Self::emit_ok(env.contract.address, ica.host().clone());
        let next_state = self.connectee.connected(ica);
        next_state
            .enter(env.block.time.into_instant(), querier)
            .map(|batch| message::Response::messages_with_event(batch, event))
            .map(|cw_resp| Response::<Self>::from(cw_resp, next_state))
    }
}

impl<Connectee, SwapResult> Contract for IcaConnector<Connectee, SwapResult>
where
    Connectee: Contract,
{
    type StateResponse = Connectee::StateResponse;

    fn state(
        self,
        now: Instant,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        self.connectee.state(now, due_projection, querier)
    }
}

impl<Connectee, SwapResult> Display for IcaConnector<Connectee, SwapResult>
where
    Connectee: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!("IcaConnector({})", self.connectee))
    }
}

impl<Connectee, SwapResult> TimeAlarm for IcaConnector<Connectee, SwapResult>
where
    Connectee: TimeAlarm,
{
    fn setup_alarm(&self, r#for: Instant) -> Result<Batch> {
        self.connectee.setup_alarm(r#for)
    }
}
