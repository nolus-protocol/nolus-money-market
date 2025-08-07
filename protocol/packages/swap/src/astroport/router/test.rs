use crate::astroport::router::Router;

pub struct TestRouter {}

impl Router for TestRouter {
    /// Source: https://github.com/astroport-fi/astroport-changelog/blob/main/neutron/pion-1/core_testnet.json
    const ADDRESS: &'static str =
        "neutron12jm24l9lr9cupufqjuxpdjnnweana4h66tsx5cl800mke26td26sq7m05p";
}
