use sdk::cosmwasm_std::{MessageInfo, QuerierWrapper};

use crate::{Account, Enterable, error::Result as DexResult};

/// Entity expecting to be connected to ICA
pub trait IcaConnectee {
    type State;
    type NextState: Enterable + Into<Self::State>;

    fn connected(self, ica_account: Account) -> Self::NextState;

    /// Authorise an inbound `RemoteLeaseCallback` against this
    /// connectee's owning contract. Connectees decide internally what
    /// "authorised" means; those that do not participate in the
    /// remote-lease protocol reject with `Error::Unauthorized`.
    fn authz_remote_callback(
        &self,
        querier: QuerierWrapper<'_>,
        info: &MessageInfo,
    ) -> DexResult<()>;
}
