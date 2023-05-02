use lease::api::MigrateMsg;
use platform::batch::Batch;
use sdk::cosmwasm_std::Addr;

use crate::{error::ContractError, result::ContractResult};

pub fn migrate_leases<I>(leases: I, lease_code_id: u64) -> ContractResult<Batch>
where
    I: Iterator<Item = ContractResult<Addr>>,
{
    let no_msgs = MigrateBatch::new(lease_code_id);
    let migrated_msgs = leases.fold(no_msgs, MigrateBatch::migrate_lease);
    migrated_msgs.try_into()
}

struct MigrateBatch {
    new_code_id: u64,
    may_batch: ContractResult<Batch>,
}
impl MigrateBatch {
    fn new(new_code_id: u64) -> Self {
        Self {
            new_code_id,
            may_batch: Ok(Batch::default()),
        }
    }

    fn migrate_lease(mut self, lease_contract: ContractResult<Addr>) -> Self {
        let op = |mut batch: Batch| {
            lease_contract.and_then(|lease| {
                batch
                    .schedule_migrate_wasm_no_reply(&lease, MigrateMsg {}, self.new_code_id)
                    .map(|_| batch)
                    .map_err(Into::into)
            })
        };

        self.may_batch = self.may_batch.and_then(op);
        self
    }
}

impl TryFrom<MigrateBatch> for Batch {
    type Error = ContractError;
    fn try_from(this: MigrateBatch) -> Result<Self, Self::Error> {
        this.may_batch
    }
}

#[cfg(test)]
mod test {
    use lease::api::MigrateMsg;
    use platform::batch::Batch;
    use sdk::cosmwasm_std::Addr;

    use crate::ContractError;

    #[test]
    fn no_leases() {
        let new_code = 242;
        let no_leases = vec![];
        assert_eq!(
            Ok(Batch::default()),
            super::migrate_leases(no_leases.into_iter().map(Ok), new_code)
        );
    }

    #[test]
    fn err_leases() {
        let new_code = 242;
        let err = "testing error";

        let no_leases = vec![
            Ok(Addr::unchecked("fsdffg")),
            Err(ContractError::ParseError { err: err.into() }),
            Ok(Addr::unchecked("2424")),
        ];
        assert_eq!(
            Err(ContractError::ParseError { err: err.into() }),
            super::migrate_leases(no_leases.into_iter(), new_code)
        );
    }

    #[test]
    fn a_few_leases() {
        let new_code = 242;
        let addr1 = Addr::unchecked("11111");
        let addr2 = Addr::unchecked("22222");
        let leases = vec![addr1.clone(), addr2.clone()];

        let mut exp = Batch::default();
        exp.schedule_migrate_wasm_no_reply(&addr1, MigrateMsg {}, new_code)
            .unwrap();
        exp.schedule_migrate_wasm_no_reply(&addr2, MigrateMsg {}, new_code)
            .unwrap();

        assert_eq!(
            Ok(exp),
            super::migrate_leases(leases.into_iter().map(Ok), new_code)
        );
    }
}
