use super::{
    super::higher_order_type::{Map, MapAsRef, TryForEach},
    Granularity, HigherOrderType,
};

impl<StructuralTypeConstructor, GranularUnitWrapperTypeConstructor> TryForEach
    for HigherOrderType<StructuralTypeConstructor, GranularUnitWrapperTypeConstructor>
where
    StructuralTypeConstructor: TryForEach,
    GranularUnitWrapperTypeConstructor: TryForEach,
{
    fn try_for_each<Unit, F, Err>(this: Self::Of<Unit>, mut f: F) -> Result<(), Err>
    where
        F: FnMut(Unit) -> Result<(), Err>,
    {
        match this {
            Granularity::Some { some: fine_grain } => {
                StructuralTypeConstructor::try_for_each(fine_grain, move |fine_grain| {
                    GranularUnitWrapperTypeConstructor::try_for_each(fine_grain, &mut f)
                })
            }
            Granularity::All(coarse_grain) => GranularUnitWrapperTypeConstructor::try_for_each(
                coarse_grain,
                move |coarse_grain| StructuralTypeConstructor::try_for_each(coarse_grain, &mut f),
            ),
        }
    }
}

impl<StructuralTypeConstructor, GranularUnitWrapperTypeConstructor> Map
    for HigherOrderType<StructuralTypeConstructor, GranularUnitWrapperTypeConstructor>
where
    StructuralTypeConstructor: Map,
    GranularUnitWrapperTypeConstructor: Map,
{
    fn map<Unit, F, MappedUnit>(this: Self::Of<Unit>, mut f: F) -> Self::Of<MappedUnit>
    where
        F: FnMut(Unit) -> MappedUnit,
    {
        match this {
            Granularity::Some { some: fine_grain } => Granularity::Some {
                some: StructuralTypeConstructor::map(fine_grain, move |fine_grain| {
                    GranularUnitWrapperTypeConstructor::map(fine_grain, &mut f)
                }),
            },
            Granularity::All(coarse_grain) => Granularity::All(
                GranularUnitWrapperTypeConstructor::map(coarse_grain, move |coarse_grain| {
                    StructuralTypeConstructor::map(coarse_grain, &mut f)
                }),
            ),
        }
    }
}

impl<StructuralTypeConstructor, GranularUnitWrapperTypeConstructor> MapAsRef
    for HigherOrderType<StructuralTypeConstructor, GranularUnitWrapperTypeConstructor>
where
    StructuralTypeConstructor: Map + MapAsRef,
    GranularUnitWrapperTypeConstructor: Map + MapAsRef,
{
    fn map_as_ref<Unit>(this: &Self::Of<Unit>) -> Self::Of<&Unit> {
        match this {
            Granularity::Some { some: fine_grain } => Granularity::Some {
                some: StructuralTypeConstructor::map(
                    StructuralTypeConstructor::map_as_ref(fine_grain),
                    GranularUnitWrapperTypeConstructor::map_as_ref,
                ),
            },
            Granularity::All(coarse_grain) => {
                Granularity::All(GranularUnitWrapperTypeConstructor::map(
                    GranularUnitWrapperTypeConstructor::map_as_ref(coarse_grain),
                    StructuralTypeConstructor::map_as_ref,
                ))
            }
        }
    }
}
