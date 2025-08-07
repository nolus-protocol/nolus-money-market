use crate::astroport::router::Router;

pub struct MainRouter {}

impl Router for MainRouter {
    /// Source: https://github.com/astroport-fi/astroport-changelog/blob/main/neutron/neutron-1/core_mainnet.json
    const ADDRESS: &'static str =
        "neutron1rwj6mfxzzrwskur73v326xwuff52vygqk73lr7azkehnfzz5f5wskwekf4";
}
