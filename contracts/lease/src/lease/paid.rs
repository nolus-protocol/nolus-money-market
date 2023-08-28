use std::marker::PhantomData;

use currency::Currency;
use finance::coin::Coin;
use platform::{bank::BankAccount, batch::Batch};
use sdk::cosmwasm_std::Addr;

use crate::error::ContractResult;

use super::LeaseDTO;

pub struct Lease<Asset, Lpn> {
    customer: Addr,
    amount: Coin<Asset>,
    lpn: PhantomData<Lpn>,
}

impl<Asset, Lpn> Lease<Asset, Lpn>
where
    Asset: Currency,
    Lpn: Currency,
{
    pub(crate) fn from_dto(dto: LeaseDTO) -> Self {
        let amount = dto.amount.try_into().expect(
            "The DTO -> Lease conversion should have resulted in Asset == dto.amount.symbol()",
        );
        Self {
            customer: dto.customer,
            amount,
            lpn: PhantomData,
        }
    }

    pub(crate) fn close<B>(self, mut lease_account: B) -> ContractResult<Batch>
    where
        B: BankAccount,
    {
        let surplus = lease_account.balance::<Lpn>()?;

        if !surplus.is_zero() {
            lease_account.send(surplus, &self.customer);
        }

        lease_account.send(self.amount, &self.customer);

        Ok(lease_account.into())
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use currency::{lease::Atom, lpn::Usdc, Currency, Group};
    use finance::{
        coin::{Coin, WithCoin},
        zero::Zero,
    };
    use platform::{
        bank::{self, Aggregate, BalancesResult, BankAccountView, BankStub},
        batch::Batch,
        error::Result as PlatformResult,
    };
    use sdk::cosmwasm_std::Addr;

    use super::Lease;

    const CUSTOMER: &str = "customer";
    type TestLpn = Usdc;
    type TestAsset = Atom;

    pub struct MockBankView {
        balance: Coin<TestAsset>,
        balance_surplus: Coin<TestLpn>,
    }

    impl MockBankView {
        fn new(amount: Coin<TestAsset>, amount_surplus: Coin<TestLpn>) -> Self {
            Self {
                balance: amount,
                balance_surplus: amount_surplus,
            }
        }
        fn only_balance(amount: Coin<TestAsset>) -> Self {
            Self {
                balance: amount,
                balance_surplus: Coin::ZERO,
            }
        }
    }

    impl BankAccountView for MockBankView {
        fn balance<C>(&self) -> PlatformResult<Coin<C>>
        where
            C: Currency,
        {
            if currency::equal::<C, TestAsset>() {
                Ok(Coin::<C>::new(self.balance.into()))
            } else if currency::equal::<C, TestLpn>() {
                Ok(Coin::<C>::new(self.balance_surplus.into()))
            } else {
                unreachable!("Expected {}, found {}", TestAsset::TICKER, C::TICKER);
            }
        }

        fn balances<G, Cmd>(&self, _: Cmd) -> BalancesResult<Cmd>
        where
            G: Group,
            Cmd: WithCoin,
            Cmd::Output: Aggregate,
        {
            unimplemented!()
        }
    }

    pub fn create_lease<Asset, Lpn>(amount: Coin<Asset>) -> Lease<Asset, Lpn>
    where
        Lpn: Currency,
        Asset: Currency,
    {
        Lease {
            customer: Addr::unchecked(CUSTOMER),
            amount,
            lpn: PhantomData,
        }
    }

    #[test]
    fn close_no_surplus() {
        let lease_amount = 10.into();
        let lease: Lease<TestAsset, TestLpn> = create_lease(lease_amount);
        let lease_account = BankStub::new(MockBankView::only_balance(lease_amount));
        let res = lease.close(lease_account).unwrap();
        assert_eq!(
            res,
            bank::bank_send(Batch::default(), CUSTOMER, lease_amount)
        );
    }

    #[test]
    fn close_with_surplus() {
        let lease_amount = 10.into();
        let surplus_amount = 2.into();
        let lease: Lease<TestAsset, TestLpn> = create_lease(lease_amount);
        let lease_account = BankStub::new(MockBankView::new(lease_amount, surplus_amount));
        let res = lease.close(lease_account).unwrap();
        assert_eq!(res, {
            bank::bank_send(
                bank::bank_send(Batch::default(), CUSTOMER, surplus_amount),
                CUSTOMER,
                lease_amount,
            )
        });
    }
}
