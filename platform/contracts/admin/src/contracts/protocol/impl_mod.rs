use platform::batch::Batch;
use sdk::cosmwasm_std::{Addr, QuerierWrapper};
use versioning::{ProtocolPackageRelease, ProtocolPackageReleaseId};

use crate::{result::Result, validate::Validate};

use super::{
    super::{
        MigrationSpec,
        higher_order_type::{Map, MapAsRef, TryForEach, TryForEachPair as _, Zip},
        impl_mod::migrate_contract,
    },
    Contracts, Protocol, higher_order_type,
};

impl Contracts<Addr> {
    pub(crate) fn migrate_standalone(
        self,
        querier: QuerierWrapper<'_>,
        to_release: ProtocolPackageReleaseId,
        migration_msgs: Contracts<MigrationSpec>,
    ) -> Result<Batch> {
        let mut migration_batch = Batch::default();

        let mut post_migration_execute_batch = Batch::default();

        higher_order_type::Contracts::try_for_each_pair(
            self,
            migration_msgs,
            |address, migration_spec| {
                migrate_contract::<ProtocolPackageRelease>(
                    querier,
                    &mut migration_batch,
                    &mut post_migration_execute_batch,
                    address,
                    to_release.clone(),
                    migration_spec,
                )
            },
        )
        .map(|()| migration_batch.merge(post_migration_execute_batch))
    }
}

impl<T> Validate for Contracts<T>
where
    T: Validate,
{
    type Context<'r> = T::Context<'r>;

    type Error = T::Error;

    fn validate(&self, ctx: Self::Context<'_>) -> Result<(), Self::Error> {
        higher_order_type::Contracts::try_for_each(
            higher_order_type::Contracts::map_as_ref(self),
            |contract| contract.validate(ctx),
        )
    }
}

impl TryForEach for higher_order_type::Contracts {
    #[inline]
    fn try_for_each<Unit, F, Err>(this: Self::Of<Unit>, f: F) -> std::result::Result<(), Err>
    where
        F: FnMut(Unit) -> std::result::Result<(), Err>,
    {
        [
            this.leaser,
            this.lpp,
            this.oracle,
            this.profit,
            this.reserve,
        ]
        .into_iter()
        .try_for_each(f)
    }
}

impl Map for higher_order_type::Contracts {
    fn map<Unit, F, MappedUnit>(this: Self::Of<Unit>, mut f: F) -> Self::Of<MappedUnit>
    where
        F: FnMut(Unit) -> MappedUnit,
    {
        Contracts {
            leaser: f(this.leaser),
            lpp: f(this.lpp),
            oracle: f(this.oracle),
            profit: f(this.profit),
            reserve: f(this.reserve),
        }
    }
}

impl MapAsRef for higher_order_type::Contracts {
    #[inline]
    fn map_as_ref<T>(this: &Self::Of<T>) -> Self::Of<&T> {
        Contracts {
            leaser: &this.leaser,
            lpp: &this.lpp,
            oracle: &this.oracle,
            profit: &this.profit,
            reserve: &this.reserve,
        }
    }
}

impl Zip for higher_order_type::Contracts {
    #[inline]
    fn zip<LeftUnit, RightUnit>(
        left: Self::Of<LeftUnit>,
        right: Self::Of<RightUnit>,
    ) -> Self::Of<(LeftUnit, RightUnit)> {
        Contracts {
            leaser: (left.leaser, right.leaser),
            lpp: (left.lpp, right.lpp),
            oracle: (left.oracle, right.oracle),
            profit: (left.profit, right.profit),
            reserve: (left.reserve, right.reserve),
        }
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
