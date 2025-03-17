use sdk::cosmwasm_std::{Addr, QuerierWrapper};
use versioning::{ProtocolPackageRelease, UpdatablePackage};

use crate::{ContractError, result::ContractResult};

pub(crate) fn query_release(
    querier: QuerierWrapper<'_>,
    lease: Addr,
) -> ContractResult<ProtocolPackageRelease> {
    querier
        .query_wasm_smart(lease, &ProtocolPackageRelease::VERSION_QUERY)
        .map_err(ContractError::QueryLeasePackage)
}
