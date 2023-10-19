use std::env::{self, VarError};

const TEST_NET: &str = "dev";
const TEST_DEX: &str = "osmosis";

fn main() {
    if option_env("CARGO_FEATURE_TESTING").is_some() {
        enable_net_to_dex(TEST_NET, TEST_DEX)
    } else if option_env("CARGO_FEATURE_IMPL").is_some() {
        enable_net_to_dex(
            &option_env("NET").expect("NET=[dex|test|main] should be set"),
            &option_env("DEX").expect("DEX=osmosis should be set"),
        )
    }
}

fn option_env(key: &str) -> Option<String> {
    match env::var(key) {
        Ok(value) => Some(value),
        Err(VarError::NotPresent) => None,
        Err(_) => panic!("invalid environment value for '{}'", key),
    }
}

fn enable_net_to_dex(net: &str, dex: &str) {
    println!("cargo:rustc-cfg=net=\"{}\"", net);
    println!("cargo:rustc-cfg=dex=\"{}\"", dex);
}
