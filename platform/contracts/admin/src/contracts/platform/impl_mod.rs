use crate::validate::Validate;

use super::{
    super::higher_order_type::{Map, MapAsRef, TryForEach, Zip},
    higher_order_type::{Contracts, ContractsWithoutAdmin},
};

impl<T> Validate for super::ContractsWithoutAdmin<T>
where
    T: Validate,
{
    type Context<'r> = T::Context<'r>;

    type Error = T::Error;

    fn validate(&self, ctx: Self::Context<'_>) -> Result<(), Self::Error> {
        ContractsWithoutAdmin::try_for_each(ContractsWithoutAdmin::map_as_ref(self), |contract| {
            contract.validate(ctx)
        })
    }
}

impl<T> Validate for super::Contracts<T>
where
    T: Validate,
{
    type Context<'r> = T::Context<'r>;

    type Error = T::Error;

    fn validate(&self, ctx: Self::Context<'_>) -> Result<(), Self::Error> {
        Contracts::try_for_each(Contracts::map_as_ref(self), |contract| {
            contract.validate(ctx)
        })
    }
}

impl TryForEach for ContractsWithoutAdmin {
    #[inline]
    fn try_for_each<Unit, F, Err>(this: Self::Of<Unit>, f: F) -> Result<(), Err>
    where
        F: FnMut(Unit) -> Result<(), Err>,
    {
        let super::ContractsWithoutAdmin {
            timealarms,
            treasury,
        } = this;

        [timealarms, treasury].into_iter().try_for_each(f)
    }
}

impl Map for ContractsWithoutAdmin {
    #[inline]
    fn map<Unit, F, MappedUnit>(this: Self::Of<Unit>, mut f: F) -> Self::Of<MappedUnit>
    where
        F: FnMut(Unit) -> MappedUnit,
    {
        super::ContractsWithoutAdmin {
            timealarms: f(this.timealarms),
            treasury: f(this.treasury),
        }
    }
}

impl MapAsRef for ContractsWithoutAdmin {
    #[inline]
    fn map_as_ref<T>(this: &Self::Of<T>) -> Self::Of<&T> {
        super::ContractsWithoutAdmin {
            timealarms: &this.timealarms,
            treasury: &this.treasury,
        }
    }
}

impl Zip for ContractsWithoutAdmin {
    #[inline]
    fn zip<LeftUnit, RightUnit>(
        left: Self::Of<LeftUnit>,
        right: Self::Of<RightUnit>,
    ) -> Self::Of<(LeftUnit, RightUnit)> {
        super::ContractsWithoutAdmin {
            timealarms: (left.timealarms, right.timealarms),
            treasury: (left.treasury, right.treasury),
        }
    }
}

impl TryForEach for Contracts {
    #[inline]
    fn try_for_each<Unit, F, Err>(this: Self::Of<Unit>, f: F) -> Result<(), Err>
    where
        F: FnMut(Unit) -> Result<(), Err>,
    {
        let super::Contracts {
            admin,
            timealarms,
            treasury,
        } = this;

        [admin, timealarms, treasury].into_iter().try_for_each(f)
    }
}

impl Map for Contracts {
    #[inline]
    fn map<Unit, F, MappedUnit>(this: Self::Of<Unit>, mut f: F) -> Self::Of<MappedUnit>
    where
        F: FnMut(Unit) -> MappedUnit,
    {
        super::Contracts {
            admin: f(this.admin),
            timealarms: f(this.timealarms),
            treasury: f(this.treasury),
        }
    }
}

impl MapAsRef for Contracts {
    #[inline]
    fn map_as_ref<T>(this: &Self::Of<T>) -> Self::Of<&T> {
        super::Contracts {
            admin: &this.admin,
            timealarms: &this.timealarms,
            treasury: &this.treasury,
        }
    }
}

impl Zip for Contracts {
    #[inline]
    fn zip<LeftUnit, RightUnit>(
        left: Self::Of<LeftUnit>,
        right: Self::Of<RightUnit>,
    ) -> Self::Of<(LeftUnit, RightUnit)> {
        super::Contracts {
            admin: (left.admin, right.admin),
            timealarms: (left.timealarms, right.timealarms),
            treasury: (left.treasury, right.treasury),
        }
    }
}
