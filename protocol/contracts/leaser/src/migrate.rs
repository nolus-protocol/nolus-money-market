use std::iter;

use lease::api::MigrateMsg;
use platform::{batch::Batch, contract::Code};
use sdk::cosmwasm_std::Addr;
use versioning::ProtocolMigrationMessage;

use crate::{msg::MaxLeases, result::ContractResult, state::leases::CustomerWithLeasesIterator};

pub(crate) struct Customer<Leases> {
    customer: Addr,
    leases: Leases,
}

impl<Leases> Customer<Leases> {
    pub fn from(customer: Addr, leases: Leases) -> Self {
        Self { customer, leases }
    }

    fn map_leases<NewLeases, F>(self, f: F) -> Customer<NewLeases>
    where
        F: FnOnce(Leases) -> NewLeases,
    {
        let Self { customer, leases } = self;

        Customer {
            customer,
            leases: f(leases),
        }
    }
}

pub(crate) type MaybeCustomer<Leases> = ContractResult<Customer<Leases>>;

#[derive(Default)]
#[cfg_attr(feature = "testing", derive(Debug, Eq, PartialEq))]
pub(crate) struct MigrationResult {
    pub msgs: Batch,
    pub next_customer: Option<Addr>,
}

impl MigrationResult {
    pub fn try_add_msgs<F>(mut self, add_fn: F) -> ContractResult<Self>
    where
        F: FnOnce(&mut Batch) -> ContractResult<()>,
    {
        add_fn(&mut self.msgs).map(|()| self)
    }
}

/// Builds a batch of messages for the migration of up to `max_leases`
///
/// The customer connected leases are migrated up to the `max_leases` boundary and atomically for a customer.
/// If there are still pending customers, then the next customer is returned as a key to start from the next chunk of leases.
///
/// Consumes the customers iterator to the next customer or error.
pub(crate) fn migrate_leases<Customers>(
    mut customers: Customers,
    lease_code: Code,
    migrate_from: ProtocolMigrationMessage<MigrateMsg>,
    max_leases: MaxLeases,
) -> ContractResult<MigrationResult>
where
    Customers: CustomerWithLeasesIterator,
{
    let mut msgs = MigrateBatch::new(lease_code, migrate_from, max_leases);

    customers
        .find_map(|maybe_customer| match maybe_customer {
            Ok(customer) => msgs.migrate_or_be_next(customer),
            Err(err) => Some(Err(err)),
        })
        .transpose()
        .map(|next_customer| MigrationResult {
            msgs: msgs.into(),
            next_customer,
        })
}

pub(crate) fn extract_first_lease_address<Customers>(
    mut customers: Customers,
) -> Option<ContractResult<ExtractFirstLeaseAddressOutput<impl CustomerWithLeasesIterator>>>
where
    Customers: CustomerWithLeasesIterator,
{
    customers
        .find_map(|result| {
            result
                .map(|customer| {
                    let mut customer = customer.map_leases(Iterator::peekable);

                    customer
                        .leases
                        .peek()
                        .cloned()
                        .map(|lease| (customer.map_leases(either::Left), lease))
                })
                .transpose()
        })
        .map(|result| {
            result.map(
                |(customer, first_lease_address)| ExtractFirstLeaseAddressOutput {
                    customers: iter::once(Ok(customer)).chain(
                        customers.map(|result| {
                            result.map(|customer| customer.map_leases(either::Right))
                        }),
                    ),
                    first_lease_address,
                },
            )
        })
}

pub(crate) struct ExtractFirstLeaseAddressOutput<Customers> {
    pub customers: Customers,
    pub first_lease_address: Addr,
}

struct MigrateBatch {
    new_code: Code,
    migrate_from: ProtocolMigrationMessage<MigrateMsg>,
    leases_left: MaxLeases,
    msgs: Batch,
}

impl MigrateBatch {
    fn new(
        new_code: Code,
        migrate_from: ProtocolMigrationMessage<MigrateMsg>,
        max_leases: MaxLeases,
    ) -> Self {
        Self {
            new_code,
            migrate_from,
            leases_left: max_leases,
            msgs: Default::default(),
        }
    }

    /// None if there is enough room for all customer's leases, otherwise return the customer
    fn migrate_or_be_next<Leases>(
        &mut self,
        customer: Customer<Leases>,
    ) -> Option<ContractResult<Addr>>
    where
        Leases: ExactSizeIterator<Item = Addr>,
    {
        self.migrate_leases(customer.leases)
            .map(|completed| completed.map(|()| customer.customer))
    }

    /// None if there is enough capacity for all leases, Some(Ok(())) - none migrated due to less available seats, Some(Err) - if an error occurs at some point
    fn migrate_leases<Leases>(&mut self, mut leases: Leases) -> Option<ContractResult<()>>
    where
        Leases: ExactSizeIterator<Item = Addr>,
    {
        let maybe_leases_nb: Result<MaxLeases, _> = leases.len().try_into();

        match maybe_leases_nb {
            Err(err) => Some(Err(err.into())),
            Ok(leases_nb) => {
                if let Some(left) = self.leases_left.checked_sub(leases_nb) {
                    self.leases_left = left;

                    leases
                        .find_map(|lease| self.schedule_migration(lease).map(|()| None).transpose())
                } else {
                    Some(Ok(()))
                }
            }
        }
    }

    fn schedule_migration(&mut self, lease: Addr) -> ContractResult<()> {
        self.msgs
            .schedule_migrate_wasm_no_reply(lease, &self.migrate_from, self.new_code)
            .map_err(Into::into)
    }
}

impl From<MigrateBatch> for Batch {
    fn from(this: MigrateBatch) -> Self {
        this.msgs
    }
}

#[cfg(all(feature = "internal.test.testing", test))]
mod test {
    use lease::api::MigrateMsg;
    use platform::contract::Code;
    use sdk::cosmwasm_std::Addr;
    use versioning::{
        ProtocolMigrationMessage, ProtocolPackageRelease, ProtocolPackageReleaseId, ReleaseId,
    };

    use crate::error::ContractError;

    use super::{Customer, CustomerWithLeasesIterator, MigrationResult};

    const LEASE1: &str = "lease1";
    const LEASE21: &str = "lease21";
    const LEASE22: &str = "lease22";
    const LEASE3: &str = "lease3";
    const LEASE41: &str = "lease41";
    const LEASE42: &str = "lease42";
    const LEASE43: &str = "lease43";
    const CUSTOMER_ADDR1: &str = "customer1";
    const CUSTOMER_ADDR2: &str = "customer2";
    const CUSTOMER_ADDR3: &str = "customer3";
    const CUSTOMER_ADDR4: &str = "customer4";

    #[test]
    fn no_leases() {
        use std::array::IntoIter;
        let new_code = Code::unchecked(242);
        let no_leases: Vec<Customer<IntoIter<Addr, 0>>> = vec![];
        assert_eq!(
            Ok(MigrationResult::default()),
            super::migrate_leases(no_leases.into_iter().map(Ok), new_code, migrate_msg(), 2,)
        );
    }

    #[test]
    fn more_than_max_leases() {
        let new_code = Code::unchecked(242);
        let lease1 = Addr::unchecked(LEASE1);
        let lease2 = Addr::unchecked(LEASE21);
        let lease3 = Addr::unchecked(LEASE22);
        let customer_addr1 = Addr::unchecked(CUSTOMER_ADDR1);
        let cust1 = Customer::from(customer_addr1.clone(), [lease1, lease2, lease3].into_iter());

        let customers = [Ok(cust1)];
        {
            let exp = MigrationResult {
                next_customer: Some(customer_addr1),
                ..Default::default()
            };
            assert_eq!(
                Ok(exp),
                super::migrate_leases(customers.into_iter(), new_code, migrate_msg(), 2,)
            );
        }
    }

    #[test]
    fn paging() {
        let new_code = Code::unchecked(242);
        let lease1 = Addr::unchecked(LEASE1);
        let lease21 = Addr::unchecked(LEASE21);
        let lease22 = Addr::unchecked(LEASE22);
        let lease3 = Addr::unchecked(LEASE3);
        let lease41 = Addr::unchecked(LEASE41);
        let lease42 = Addr::unchecked(LEASE42);
        let lease43 = Addr::unchecked(LEASE43);
        fn customer1() -> Addr {
            Addr::unchecked(CUSTOMER_ADDR1)
        }
        fn customer2() -> Addr {
            Addr::unchecked(CUSTOMER_ADDR2)
        }
        fn customer3() -> Addr {
            Addr::unchecked(CUSTOMER_ADDR3)
        }
        fn customer4() -> Addr {
            Addr::unchecked(CUSTOMER_ADDR4)
        }

        {
            let exp = MigrationResult {
                next_customer: Some(customer1()),
                ..Default::default()
            };
            assert_eq!(
                Ok(exp),
                super::migrate_leases(test_customers(), new_code, migrate_msg(), 0,)
            );
        }
        {
            let mut exp = add_expected(MigrationResult::default(), lease1.clone(), new_code);
            exp.next_customer = Some(customer2());
            assert_eq!(
                Ok(exp),
                super::migrate_leases(test_customers(), new_code, migrate_msg(), 1,)
            );
        }
        {
            let mut exp = add_expected(MigrationResult::default(), lease1.clone(), new_code);
            exp.next_customer = Some(customer2());
            assert_eq!(
                Ok(exp),
                super::migrate_leases(test_customers(), new_code, migrate_msg(), 2,)
            );
        }
        {
            let exp = add_expected(MigrationResult::default(), lease1.clone(), new_code);
            let exp = add_expected(exp, lease21.clone(), new_code);
            let mut exp = add_expected(exp, lease22.clone(), new_code);
            exp.next_customer = Some(customer3());
            assert_eq!(
                Ok(exp),
                super::migrate_leases(test_customers(), new_code, migrate_msg(), 3,)
            );
        }
        {
            let exp = add_expected(MigrationResult::default(), lease1.clone(), new_code);
            let exp = add_expected(exp, lease21.clone(), new_code);
            let exp = add_expected(exp, lease22.clone(), new_code);
            let mut exp = add_expected(exp, lease3.clone(), new_code);
            exp.next_customer = Some(customer4());
            assert_eq!(
                Ok(exp),
                super::migrate_leases(test_customers(), new_code, migrate_msg(), 4,)
            );
        }
        {
            let exp = add_expected(MigrationResult::default(), lease1.clone(), new_code);
            let exp = add_expected(exp, lease21.clone(), new_code);
            let exp = add_expected(exp, lease22.clone(), new_code);
            let mut exp = add_expected(exp, lease3.clone(), new_code);
            exp.next_customer = Some(customer4());
            assert_eq!(
                Ok(exp),
                super::migrate_leases(test_customers(), new_code, migrate_msg(), 5,)
            );
        }
        {
            let exp = add_expected(MigrationResult::default(), lease1, new_code);
            let exp = add_expected(exp, lease21, new_code);
            let exp = add_expected(exp, lease22, new_code);
            let exp = add_expected(exp, lease3, new_code);
            let exp = add_expected(exp, lease41, new_code);
            let exp = add_expected(exp, lease42, new_code);
            let mut exp = add_expected(exp, lease43, new_code);
            exp.next_customer = None;
            assert_eq!(
                Ok(exp),
                super::migrate_leases(test_customers(), new_code, migrate_msg(), 7,)
            );
        }
    }

    #[test]
    fn err_leases() {
        let new_code = Code::unchecked(242);
        let lease1 = Addr::unchecked("lease11");
        let lease2 = Addr::unchecked("lease12");
        let lease3 = Addr::unchecked("lease13");
        let cust1 = Customer::from(
            Addr::unchecked("customer1"),
            [lease1, lease2, lease3].into_iter(),
        );
        let err = "testing error";

        let customers = [
            Ok(cust1),
            Err(ContractError::ParseError { err: err.into() }),
        ];
        assert_eq!(
            Err(ContractError::ParseError { err: err.into() }),
            super::migrate_leases(customers.into_iter(), new_code, migrate_msg(), 3,)
        );
    }

    fn add_expected(mut exp: MigrationResult, lease_addr: Addr, new_code: Code) -> MigrationResult {
        exp.msgs
            .schedule_migrate_wasm_no_reply(lease_addr.clone(), &migrate_msg(), new_code)
            .expect("Migration message should be serializable");
        exp
    }

    fn test_customers() -> impl CustomerWithLeasesIterator {
        let lease1 = Addr::unchecked(LEASE1);
        let customer_addr1 = Addr::unchecked(CUSTOMER_ADDR1);
        let cust1 = Customer::from(customer_addr1, vec![lease1].into_iter());

        let lease21 = Addr::unchecked(LEASE21);
        let lease22 = Addr::unchecked(LEASE22);
        let customer_addr2 = Addr::unchecked(CUSTOMER_ADDR2);
        let cust2 = Customer::from(customer_addr2, vec![lease21, lease22].into_iter());

        let lease3 = Addr::unchecked(LEASE3);
        let customer_addr3 = Addr::unchecked(CUSTOMER_ADDR3);
        let cust3 = Customer::from(customer_addr3, vec![lease3].into_iter());

        let lease41 = Addr::unchecked(LEASE41);
        let lease42 = Addr::unchecked(LEASE42);
        let lease43 = Addr::unchecked(LEASE43);
        let customer_addr4 = Addr::unchecked(CUSTOMER_ADDR4);
        let cust4 = Customer::from(customer_addr4, vec![lease41, lease42, lease43].into_iter());

        vec![Ok(cust1), Ok(cust2), Ok(cust3), Ok(cust4)].into_iter()
    }

    fn migrate_msg() -> ProtocolMigrationMessage<MigrateMsg> {
        ProtocolMigrationMessage {
            migrate_from: ProtocolPackageRelease::current("moduleX", "0.1.2", 1),
            to_release: ProtocolPackageReleaseId::new(
                ReleaseId::new_test("v0.7.6"),
                ReleaseId::new_test("v0.0.5"),
            ),
            message: MigrateMsg {},
        }
    }
}
