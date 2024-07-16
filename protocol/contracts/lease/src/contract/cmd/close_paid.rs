use currency::{Currency, MemberOf};
use platform::{bank::BankAccount, batch::Batch};

use crate::{
    api::LeaseAssetCurrencies,
    error::ContractError,
    lease::{with_lease_paid::WithLeaseTypes, LeaseDTO, LeasePaid},
};

pub struct Close<Bank> {
    lease_account: Bank,
}

impl<Bank> Close<Bank> {
    pub fn new(lease_account: Bank) -> Self {
        Self { lease_account }
    }
}

impl<Bank> WithLeaseTypes for Close<Bank>
where
    Bank: BankAccount,
{
    type Output = Batch;

    type Error = ContractError;

    fn exec<Asset, Lpn>(self, dto: LeaseDTO) -> Result<Self::Output, Self::Error>
    where
        Asset: Currency + MemberOf<LeaseAssetCurrencies>,
        Lpn: Currency,
    {
        LeasePaid::<Asset, Lpn>::from_dto(dto).close(self.lease_account)
    }
}
