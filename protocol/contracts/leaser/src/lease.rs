use sdk::cosmwasm_std::{Addr, QuerierWrapper};
use versioning::{ProtocolPackageRelease, UpdatablePackage};

use crate::{ContractError, result::ContractResult};

pub trait Release {
    fn package_release(&mut self, instance: Addr) -> ContractResult<ProtocolPackageRelease>;
}

pub struct CacheFirstRelease<'querier> {
    querier: QuerierWrapper<'querier>,
    cached: Option<ProtocolPackageRelease>,
}

impl<'querier> CacheFirstRelease<'querier> {
    pub fn new(querier: QuerierWrapper<'querier>) -> Self {
        Self {
            querier,
            cached: None,
        }
    }
}

impl Release for CacheFirstRelease<'_> {
    fn package_release(&mut self, instance: Addr) -> ContractResult<ProtocolPackageRelease> {
        match &self.cached {
            None => query_release(instance.clone(), self.querier).inspect(|r| {
                self.cached = Some(r.clone());
            }),
            Some(r) => Ok(r.clone()),
        }
    }
}

fn query_release(
    instance: Addr,
    querier: QuerierWrapper<'_>,
) -> ContractResult<ProtocolPackageRelease> {
    querier
        .query_wasm_smart(instance, &ProtocolPackageRelease::VERSION_QUERY)
        .map_err(ContractError::QueryLeasePackage)
}

#[cfg(all(feature = "internal.test.testing", test))]
pub mod test {
    use sdk::cosmwasm_std::Addr;
    use versioning::ProtocolPackageRelease;

    use crate::result::ContractResult;

    use super::Release;

    pub struct FixedRelease(ProtocolPackageRelease);
    impl FixedRelease {
        pub const fn with(release: ProtocolPackageRelease) -> Self {
            Self(release)
        }
    }

    impl Release for FixedRelease {
        fn package_release(&mut self, _instance: Addr) -> ContractResult<ProtocolPackageRelease> {
            Ok(self.0.clone())
        }
    }
}
