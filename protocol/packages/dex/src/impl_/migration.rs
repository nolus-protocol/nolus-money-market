pub trait MigrateSpec<SwapTask, SwapTaskNew, SEnumNew>
where
    Self: Sized,
{
    type Out;

    fn migrate_spec<MigrateFn>(self, migrate_fn: MigrateFn) -> Self::Out
    where
        MigrateFn: FnOnce(SwapTask) -> SwapTaskNew;
}
