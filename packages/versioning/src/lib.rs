use sdk::{
    cosmwasm_std::{StdError, StdResult, Storage},
    cw_storage_plus::Item,
};

pub type Version = u16;

pub type SemVer = (Version, Version, Version);

#[macro_export]
macro_rules! package_version {
    () => {{
        fn package_version() -> $crate::SemVer {
            const VERSION: &str = ::core::env!("CARGO_PKG_VERSION");

            let mut iter = VERSION.split('.');

            let major: $crate::Version = iter
                .next()
                .expect("No major segment in version string!")
                .parse()
                .expect("Major segment in version string is not a number!");
            let minor: $crate::Version = iter
                .next()
                .expect("No minor segment in version string!")
                .parse()
                .expect("Minor segment in version string is not a number!");
            let patch: $crate::Version = iter
                .next()
                .expect("No patch segment in version string!")
                .parse()
                .expect("Patch segment in version string is not a number!");

            if iter.next().is_some() {
                ::core::panic!("Unexpected fourth segment found in version string!");
            };

            (major, minor, patch)
        }

        package_version()
    }};
}

pub const COMPONENT_VERSION_ITEM: Item<'static, SemVer> = Item::new("contract_software_version");

pub const STORAGE_VERSION_ITEM: Item<'static, Version> = Item::new("contract_storage_version");

pub fn initialize<const STORAGE_VERSION: Version>(
    storage: &mut dyn Storage,
    component_version: SemVer,
) -> StdResult<()> {
    COMPONENT_VERSION_ITEM.save(storage, &component_version)?;

    STORAGE_VERSION_ITEM.save(storage, &STORAGE_VERSION)
}

pub fn upgrade_contract<
    const EXPECTED_STORAGE_VERSION: Version,
    const NEW_STORAGE_VERSION: Version,
>(
    storage: &mut dyn Storage,
    component_version: SemVer,
) -> StdResult<()> {
    STORAGE_VERSION_ITEM
        .update(storage, |version| {
            if version == EXPECTED_STORAGE_VERSION {
                Ok(NEW_STORAGE_VERSION)
            } else {
                Err(StdError::generic_err(
                    "Couldn't upgrade contract because storage version didn't match expected one!",
                ))
            }
        })
        .map(|_| ())?;

    COMPONENT_VERSION_ITEM
        .update(storage, |version| {
            if version < component_version {
                Ok(component_version)
            } else {
                Err(StdError::generic_err(
                    "Couldn't upgrade contract because version isn't monotonically increasing!",
                ))
            }
        })
        .map(|_| ())
}
