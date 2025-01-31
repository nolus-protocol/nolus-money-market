use platform::batch::Batch;
use sdk::cosmwasm_std::Addr;

use crate::validate::Validate;

use super::{
    super::{impl_mod::migrate_contract, AsRef, ForEachPair, MigrationSpec, TryForEach},
    higher_order_type, Protocol, ProtocolContracts,
};

impl ProtocolContracts<Addr> {
    pub(crate) fn migrate_standalone(
        self,
        migration_msgs: ProtocolContracts<MigrationSpec>,
    ) -> Batch {
        let mut migration_batch = Batch::default();

        let mut post_migration_execute_batch = Batch::default();

        () = self.for_each_pair(migration_msgs, |address, migration_spec| {
            () = migrate_contract(
                &mut migration_batch,
                &mut post_migration_execute_batch,
                address,
                migration_spec,
            );
        });

        migration_batch.merge(post_migration_execute_batch)
    }
}

impl<T> AsRef for ProtocolContracts<T> {
    type Item = T;

    type HigherOrderType = higher_order_type::ProtocolContracts;

    fn as_ref(&self) -> ProtocolContracts<&T> {
        ProtocolContracts {
            leaser: &self.leaser,
            lpp: &self.lpp,
            oracle: &self.oracle,
            profit: &self.profit,
            reserve: &self.reserve,
        }
    }
}

impl<T> TryForEach for ProtocolContracts<T> {
    type Item = T;

    fn try_for_each<F, Err>(self, mut f: F) -> Result<(), Err>
    where
        F: FnMut(Self::Item) -> Result<(), Err>,
    {
        f(self.leaser)
            .and_then(|()| f(self.lpp))
            .and_then(|()| f(self.oracle))
            .and_then(|()| f(self.profit))
            .and_then(|()| f(self.reserve))
    }
}

impl<T> ForEachPair for ProtocolContracts<T> {
    type Item = T;

    type HigherOrderType = higher_order_type::ProtocolContracts;

    fn for_each_pair<CounterpartUnit, F>(
        self,
        counterpart: ProtocolContracts<CounterpartUnit>,
        mut f: F,
    ) where
        F: FnMut(T, CounterpartUnit),
    {
        f(self.leaser, counterpart.leaser);

        f(self.lpp, counterpart.lpp);

        f(self.oracle, counterpart.oracle);

        f(self.profit, counterpart.profit);

        f(self.reserve, counterpart.reserve)
    }
}

impl<T> Validate for ProtocolContracts<T>
where
    T: Validate,
{
    type Context<'r> = T::Context<'r>;

    type Error = T::Error;

    fn validate(&self, ctx: Self::Context<'_>) -> Result<(), Self::Error> {
        self.as_ref()
            .try_for_each(|contract| contract.validate(ctx))
    }
}

impl<T> Validate for Protocol<T>
where
    T: Validate,
{
    type Context<'r> = T::Context<'r>;

    type Error = T::Error;

    #[inline]
    fn validate(&self, ctx: Self::Context<'_>) -> Result<(), Self::Error> {
        self.contracts.validate(ctx)
    }
}
