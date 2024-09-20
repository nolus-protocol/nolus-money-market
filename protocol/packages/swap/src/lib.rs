/// Macros for selecting appropriate implementation depending on selected
/// features.
///
/// The macros will emit a chain of all the non-`else` variants, followed by the
/// `else` branch, chosen when none of the other predicates match.
///
/// Each non-`else` branch emits the following:
/// 1. `#[cfg(not(...)]` for the predicates of all previously processed
///     branches, if any.
/// 2. `#[cfg(...)]` for the predicate of the branch processed during the
///     current iteration.
/// 3. `#[cfg(not(...))]` for the predicates of all following non-`else`
///     branches, if any.
/// 4. `#[path = ...]` attribute pointing to the provided path.
/// 5. Finally, `mod impl_mod;`.
///
/// The `else` branch emits the following:
/// 1. `#[cfg(not(...))]` for the predicates of all non-`else` branches.
/// 2. `#[cfg(...)]` for the predicate of the `else` branch.
/// 3. `#[path = ...]` attribute pointing to the provided path.
/// 4. Finally, `mod impl_mod;`.
macro_rules! impl_mod {
    (
        $(### private ### [$(cfg($($processed_cfgs:tt)+)),+])?
        else cfg($($else_cfg:tt)+) => $else_path:literal $(,)?
    ) => {
        $($(#[cfg(not($($processed_cfgs)+))])+)?
        #[cfg($($else_cfg)+)]
        #[path = $else_path]
        mod impl_mod;
    };
    (
        $(### private ### [$(cfg($($processed_cfgs:tt)+)),+])?
        cfg($($current_cfg:tt)+) => $current_path:literal,
        $(cfg($($next_cfgs:tt)+) => $next_paths:literal,)*
        else cfg($($else_cfg:tt)+) => $else_path:literal $(,)?
    ) => {
        $($(#[cfg(not($($processed_cfgs)+))])+)?
        #[cfg($($current_cfg)+)]
        $(#[cfg(not($($next_cfgs)+))])*
        #[path = $current_path]
        mod impl_mod;

        impl_mod! {
            ### private ###
            [$($(cfg($($processed_cfgs)+),)+)? cfg($($current_cfg)+)]
            $(cfg($($next_cfgs)+) => $next_paths,)*
            else cfg($($else_cfg)+) => $else_path
        }
    };
}

impl_mod! {
    cfg(any(feature = "dex-astroport_main", feature = "dex-astroport_test")) => "astroport/mod.rs",
    cfg(feature = "dex-osmosis") => "osmosis/mod.rs",
    else cfg(feature = "testing") => "test_impl.rs",
}

#[cfg(any(feature = "testing", test))]
pub mod testing;

pub type Impl = impl_mod::Impl;
