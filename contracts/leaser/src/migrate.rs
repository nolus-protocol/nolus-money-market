use sdk::{cosmwasm_ext::Response, cosmwasm_std::Addr};

use currency::native::Nls;
use lease::api::MigrateMsg;
use platform::batch::Batch;
use platform::error::Error as PlatformError;

use crate::error::ContractResult;
use crate::ContractError;

pub struct MigrateBatch {
    may_batch: ContractResult<Batch>,
}
impl MigrateBatch {
    fn migrate_lease(self, lease_contract: Addr) -> Self {
        if let Ok(mut batch) = self.may_batch {
            batch
                .schedule_execute_wasm_no_reply::<_, Nls>(&lease_contract, MigrateMsg {}, None)
                .map_or_else(Into::into, |_| batch.into())
        } else {
            self
        }
    }
}
impl FromIterator<Addr> for MigrateBatch {
    fn from_iter<T: IntoIterator<Item = Addr>>(iter: T) -> Self {
        let batch = Self {
            may_batch: ContractResult::Ok(Batch::default()),
        };
        iter.into_iter()
            .fold(batch, |batch, item| batch.migrate_lease(item))
    }
}
impl From<Batch> for MigrateBatch {
    fn from(batch: Batch) -> Self {
        Self {
            may_batch: Ok(batch),
        }
    }
}
impl From<PlatformError> for MigrateBatch {
    fn from(err: PlatformError) -> Self {
        Self {
            may_batch: Err(err.into()),
        }
    }
}
impl TryFrom<MigrateBatch> for Response {
    type Error = ContractError;
    fn try_from(batch: MigrateBatch) -> Result<Self, Self::Error> {
        batch.may_batch.map(Into::into)
    }
}
