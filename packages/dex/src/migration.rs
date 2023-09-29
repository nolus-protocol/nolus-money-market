pub trait MigrateSpec<SwapTask, SwapTaskNew, SEnumNew>
where
    Self: Sized,
{
    type Out;

    fn migrate_spec<MigrateFn>(self, migrate_fn: MigrateFn) -> Self::Out
    where
        MigrateFn: FnOnce(SwapTask) -> SwapTaskNew;
}

pub trait InspectSpec<SwapTask, R> {
    fn inspect_spec<InspectFn>(&self, inspect_fn: InspectFn) -> R
    where
        InspectFn: FnOnce(&SwapTask) -> R;
}
