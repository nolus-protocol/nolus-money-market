const RELEASE_VERSION: &str = env!(
    "RELEASE_VERSION",
    r#"No release version provided as an environment variable! Please set "RELEASE_VERSION" environment variable!"#,
);
const RELEASE_VERSION_DEV: &str = "dev-release";

pub fn release() -> &'static str {
    self::RELEASE_VERSION
}

pub(crate) fn dev_release() -> bool {
    RELEASE_VERSION == RELEASE_VERSION_DEV
}
