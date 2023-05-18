use lease::api::MigrateMsg;
use platform::batch::Batch;
use sdk::cosmwasm_std::Addr;

use crate::{error::ContractError, msg::NbInstances, result::ContractResult};

#[derive(Default)]
#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
pub struct MigrationResult {
    pub msgs: Batch,
    /// None if the number of processed instances is less than the `max_leases`
    pub last_instance: Option<Addr>,
}

pub fn migrate_leases<I>(
    leases: I,
    lease_code_id: u64,
    max_leases: NbInstances,
) -> ContractResult<MigrationResult>
where
    I: Iterator<Item = ContractResult<Addr>>,
{
    let no_msgs = MigrateBatch::new(lease_code_id, max_leases);
    let migrated_msgs = leases
        .take(max_leases.try_into()?)
        .fold(no_msgs, MigrateBatch::migrate_lease);
    migrated_msgs.try_into()
}

impl MigrationResult {
    pub fn try_add_msgs<F>(mut self, add_fn: F) -> ContractResult<Self>
    where
        F: FnOnce(&mut Batch) -> ContractResult<()>,
    {
        add_fn(&mut self.msgs).map(|()| self)
    }
}
struct MigrateBatch {
    new_code_id: u64,
    leases_left: NbInstances,
    may_result: ContractResult<MigrationResult>,
}
impl MigrateBatch {
    fn new(new_code_id: u64, max_leases: NbInstances) -> Self {
        Self {
            new_code_id,
            leases_left: max_leases,
            may_result: Ok(Default::default()),
        }
    }

    fn migrate_lease(mut self, lease_contract: ContractResult<Addr>) -> Self {
        let op = |result: MigrationResult| {
            lease_contract.and_then(|lease| {
                self.leases_left -= 1;
                result
                    .try_add_msgs(|msgs| {
                        msgs.schedule_migrate_wasm_no_reply(&lease, MigrateMsg {}, self.new_code_id)
                            .map_err(Into::into)
                    })
                    .map(|mut res| {
                        if self.leases_left == NbInstances::MIN {
                            res.last_instance = Some(lease);
                        }
                        res
                    })
            })
        };

        self.may_result = self.may_result.and_then(op);
        self
    }
}

impl TryFrom<MigrateBatch> for MigrationResult {
    type Error = ContractError;
    fn try_from(this: MigrateBatch) -> Result<Self, Self::Error> {
        this.may_result
    }
}

#[cfg(test)]
mod test {
    use lease::api::MigrateMsg;
    use sdk::cosmwasm_std::Addr;

    use crate::{migrate::MigrationResult, ContractError};

    #[test]
    fn no_leases() {
        let new_code = 242;
        let no_leases = vec![];
        assert_eq!(
            Ok(MigrationResult::default()),
            super::migrate_leases(no_leases.into_iter().map(Ok), 2, new_code)
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
            super::migrate_leases(no_leases.into_iter(), 5, new_code)
        );
    }

    #[test]
    fn a_few_leases() {
        let new_code = 242;
        let addr1 = Addr::unchecked("11111");
        let addr2 = Addr::unchecked("22222");
        let leases = vec![addr1.clone(), addr2.clone()];

        let exp = add_expected(MigrationResult::default(), &addr1, new_code);
        let mut exp = add_expected(exp, &addr2, new_code);
        exp.last_instance = Some(addr2);

        assert_eq!(
            Ok(exp),
            super::migrate_leases(leases.into_iter().map(Ok), new_code, 2)
        );
    }

    #[test]
    fn paging() {
        let new_code = 242;
        let addr1 = Addr::unchecked("11111");
        let addr2 = Addr::unchecked("22222");
        let addr3 = Addr::unchecked("333333333");
        let addr4 = Addr::unchecked("4");
        let addr5 = Addr::unchecked("5555");
        let addr6 = Addr::unchecked("6");
        let addr7 = Addr::unchecked("777");
        let leases: Vec<Addr> = [&addr1, &addr2, &addr3, &addr4, &addr5, &addr6, &addr7]
            .map(Clone::clone)
            .into();

        {
            let exp = add_expected(MigrationResult::default(), &addr1, new_code);
            let mut exp = add_expected(exp, &addr2, new_code);
            exp.last_instance = Some(addr2);
            assert_eq!(
                Ok(exp),
                super::migrate_leases(leases.clone().into_iter().map(Ok), new_code, 2)
            );
        }

        {
            let exp = add_expected(MigrationResult::default(), &addr3, new_code);
            let exp = add_expected(exp, &addr4, new_code);
            let mut exp = add_expected(exp, &addr5, new_code);
            exp.last_instance = Some(addr5);
            assert_eq!(
                Ok(exp),
                super::migrate_leases(leases.clone().into_iter().skip(2).map(Ok), new_code, 3)
            );
        }

        {
            let exp = add_expected(MigrationResult::default(), &addr6, new_code);
            let mut exp = add_expected(exp, &addr7, new_code);
            exp.last_instance = None;
            assert_eq!(
                Ok(exp),
                super::migrate_leases(leases.into_iter().take(2 + 3).map(Ok), new_code, 15)
            );
        }
    }

    fn add_expected(mut exp: MigrationResult, lease_addr: &Addr, new_code: u64) -> MigrationResult {
        exp.msgs
            .schedule_migrate_wasm_no_reply(lease_addr, MigrateMsg {}, new_code)
            .unwrap();
        exp
    }
}
