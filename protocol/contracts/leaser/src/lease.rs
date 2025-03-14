use sdk::cosmwasm_std::{Addr, QuerierWrapper};
use versioning::{ProtocolPackageRelease, UpdatablePackage};

use crate::{error::ContractError, result::ContractResult};

pub(crate) fn query_release(
    querier: QuerierWrapper<'_>,
) -> impl FnOnce(Addr) -> ContractResult<ProtocolPackageRelease> {
    move |lease| {
        querier
            .query_wasm_smart(lease, &ProtocolPackageRelease::VERSION_QUERY)
            .map_err(ContractError::QueryLeasePackage)
    }
}
