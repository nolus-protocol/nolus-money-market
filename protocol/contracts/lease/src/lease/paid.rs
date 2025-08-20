use std::marker::PhantomData;

use currency::{Currency, CurrencyDef, MemberOf};
use platform::{bank::BankAccount, batch::Batch};
use sdk::cosmwasm_std::Addr;

use crate::{
    api::LeaseAssetCurrencies, error::ContractResult, finance::LpnCurrencies, position::Position,
};

use super::LeaseDTO;

pub struct Lease<Asset, Lpn> {
    customer: Addr,
    position: Position<Asset>,
    lpn: PhantomData<Lpn>,
}

impl<Asset, Lpn> Lease<Asset, Lpn>
where
    Asset: Currency + MemberOf<LeaseAssetCurrencies>,
{
    pub(crate) fn from_dto(dto: LeaseDTO, position: Position<Asset>) -> Self {
        Self {
            customer: dto.customer,
            position,
            lpn: PhantomData,
        }
    }
}

impl<Asset, Lpn> Lease<Asset, Lpn>
where
    Asset: CurrencyDef,
    Lpn: CurrencyDef,
    Lpn::Group: MemberOf<LpnCurrencies>,
{
    pub(crate) fn close<B>(self, mut lease_account: B) -> ContractResult<Batch>
    where
        B: BankAccount,
    {
        let surplus = lease_account.balance::<Lpn>()?;

        if !surplus.is_zero() {
            lease_account.send(surplus, self.customer.clone());
        }

        lease_account.send(self.position.amount(), self.customer);

        Ok(lease_account.into())
    }
}

#[cfg(all(feature = "internal.test.contract", test))]
mod tests {
    use std::marker::PhantomData;

    use currencies::{Lpn, testing::PaymentC3};
    use currency::Currency;
    use finance::{coin::Coin, duration::Duration, liability::Liability, percent::Percent100};
    use platform::{
        bank::{
            self, FixedAddressSender, LazySenderStub, testing as bank_testing,
            testing::MockBankView,
        },
        batch::Batch,
    };
    use sdk::cosmwasm_std::Addr;

    use crate::{
        lease::tests,
        position::{Position, Spec},
    };

    use super::Lease;

    const CUSTOMER: &str = "customer";
    type TestLpn = Lpn;
    type TestAsset = PaymentC3;

    fn create_lease<Asset, Lpn>(amount: Coin<Asset>) -> Lease<Asset, Lpn>
    where
        Asset: Currency,
    {
        let liability = Liability::new(
            Percent100::from_percent(65),
            Percent100::from_percent(70),
            Percent100::from_percent(72),
            Percent100::from_percent(74),
            Percent100::from_percent(77),
            Percent100::from_percent(80),
            Duration::from_days(3),
        );
        let spec = Spec::no_close(
            liability,
            tests::lpn_coin(15_000_000),
            tests::lpn_coin(10_000),
        );

        Lease {
            customer: Addr::unchecked(CUSTOMER),
            position: Position::new(amount, spec),
            lpn: PhantomData,
        }
    }

    #[test]
    fn close_no_surplus() {
        let lease_amount = Coin::new(10);
        let lease: Lease<TestAsset, TestLpn> = create_lease(lease_amount);
        let lease_account = bank_testing::one_transfer(
            lease_amount,
            Addr::unchecked(CUSTOMER),
            MockBankView::<_, TestLpn>::only_balance(lease_amount),
        );
        let res = lease.close(lease_account).unwrap();
        assert_eq!(
            res,
            bank::bank_send(Addr::unchecked(CUSTOMER), lease_amount)
        );
    }

    #[test]
    fn close_with_surplus() {
        let customer = Addr::unchecked(CUSTOMER);
        let lease_amount = Coin::new(10);
        let surplus_amount = tests::lpn_coin(2);
        let lease: Lease<TestAsset, TestLpn> = create_lease(lease_amount);
        let lease_account = bank_testing::two_transfers(
            surplus_amount,
            customer.clone(),
            lease_amount,
            customer.clone(),
            MockBankView::new(lease_amount, surplus_amount),
        );
        let res = lease.close(lease_account).unwrap();
        assert_eq!(res, {
            {
                let mut sender = LazySenderStub::new(customer.clone());
                sender.send(surplus_amount);
                Batch::from(sender)
            }
            .merge({
                let mut sender = LazySenderStub::new(customer);
                sender.send(lease_amount);
                sender.into()
            })
        });
    }
}
