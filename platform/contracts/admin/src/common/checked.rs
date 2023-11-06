use serde::{Deserialize, Serialize};

use platform::never::Never;
pub(crate) use sdk::cosmwasm_std::Addr as UncheckedAddr;
use sdk::cosmwasm_std::QuerierWrapper;

use crate::error::Error;

use super::transform::Transform;

impl Transform for UncheckedAddr {
    type Context<'r> = QuerierWrapper<'r>;

    type Output = Addr;

    type Error = Error;

    fn transform(self, ctx: &Self::Context<'_>) -> Result<Self::Output, Self::Error> {
        platform::contract::validate_addr(ctx, &self)
            .map(|()| Addr(self))
            .map_err(Error::Platform)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct Addr(UncheckedAddr);

impl Transform for Addr {
    type Context<'r> = ();

    type Output = StoredAddr;

    type Error = Never;

    fn transform(self, (): &Self::Context<'_>) -> Result<Self::Output, Self::Error> {
        Ok(StoredAddr(self.0))
    }
}

impl AsRef<UncheckedAddr> for Addr {
    fn as_ref(&self) -> &UncheckedAddr {
        &self.0
    }
}

impl From<Addr> for UncheckedAddr {
    fn from(value: Addr) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct StoredAddr(UncheckedAddr);

impl Transform for StoredAddr {
    type Context<'r> = ();

    type Output = Addr;

    type Error = Never;

    fn transform(self, (): &Self::Context<'_>) -> Result<Self::Output, Self::Error> {
        Ok(Addr(self.0))
    }
}
