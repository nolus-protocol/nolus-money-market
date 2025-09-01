use serde::Serialize;
use thiserror::Error;

use sdk::cosmwasm_std::{Addr, QuerierWrapper, StdError as CwError};

use super::{PlatformPackageRelease, ProtocolPackageRelease};

/// A common versioning API of each platform package
#[derive(Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum PlatformPackage {
    /// Query the platform package for its release.
    ///
    /// The result is [versioning::PlatformPackageRelease]
    #[serde(rename = "platform_package_release")]
    Release {},
}

/// A common versioning API of each protocol package
#[derive(Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ProtocolPackage {
    /// Query the protocol package for its release.
    ///
    /// The result is [versioning::ProtocolPackageRelease]
    #[serde(rename = "protocol_package_release")]
    Release {},
}

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Versioning][Query] {0}")]
    Transmission(String),
}

pub fn platform_release(
    contract: Addr,
    querier: QuerierWrapper<'_>,
) -> Result<PlatformPackageRelease, Error> {
    querier
        .query_wasm_smart(contract, &PlatformPackage::Release {})
        .map_err(|error: CwError| Error::Transmission(error.to_string()))
}

pub fn protocol_release(
    contract: Addr,
    querier: QuerierWrapper<'_>,
) -> Result<ProtocolPackageRelease, Error> {
    querier
        .query_wasm_smart(contract, &ProtocolPackage::Release {})
        .map_err(|error: CwError| Error::Transmission(error.to_string()))
}
