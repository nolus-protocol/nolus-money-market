use std::env::{self, VarError};

const TEST_NET: &str = "dev";
const TEST_DEX: &str = "osmosis";

fn main() {
    if option_env("CARGO_CFG_TEST").is_some() || option_env("CARGO_FEATURE_TESTING").is_some() {
        enable_net_to_dex(TEST_NET, TEST_DEX)
    } else if option_env("CARGO_FEATURE_IMPL").is_some() {
        enable_net_to_dex(&env::var("NET").unwrap(), &env::var("DEX").unwrap())
    }
}

fn option_env(key: &str) -> Option<String> {
    match env::var(key) {
        Ok(value) => Some(value),
        Err(VarError::NotPresent) => None,
        Err(_) => panic!("invalid environment {}", key),
    }
}
fn enable_net_to_dex(net: &str, dex: &str) {
    println!("cargo:rustc-cfg=net=\"{}\"", net);
    println!("cargo:rustc-cfg=dex=\"{}\"", dex);
}
