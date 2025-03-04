use currency::{CurrencyDef, MemberOf};
use platform::{bank::BankAccount, batch::Batch};

use crate::{
    api::LeaseAssetCurrencies,
    error::ContractError,
    finance::LpnCurrencies,
    lease::{LeaseDTO, LeasePaid, with_lease_paid::WithLeaseTypes},
    position::Position,
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

    fn exec<Asset, Lpn>(
        self,
        dto: LeaseDTO,
        position: Position<Asset>,
    ) -> Result<Self::Output, Self::Error>
    where
        Asset: CurrencyDef,
        Asset::Group: MemberOf<LeaseAssetCurrencies>,
        Lpn: CurrencyDef,
        Lpn::Group: MemberOf<LpnCurrencies>,
    {
        LeasePaid::<Asset, Lpn>::from_dto(dto, position).close(self.lease_account)
    }
}
