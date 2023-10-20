use std::env::{self, VarError};

const NET_ENV_KEY: &str = "NET";
const NET_CFG_KEY: &str = "net";
const DEX_ENV_KEY: &str = "DEX";
const DEX_CFG_KEY: &str = "dex";

const TEST_NET: &str = "dev";
const TEST_DEX: &str = "osmosis";

fn main() {
    rerun_on_env_change();
    if option_env("CARGO_FEATURE_TESTING").is_some() {
        enable_net_to_dex(TEST_NET, TEST_DEX)
    } else if option_env("CARGO_FEATURE_IMPL").is_some() {
        enable_net_to_dex(
            &option_env(NET_ENV_KEY).expect("NET=[dev|test|main] should be set"),
            &option_env(DEX_ENV_KEY).expect("DEX=osmosis should be set"),
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
    enable_cfg(NET_CFG_KEY, net);
    enable_cfg(DEX_CFG_KEY, dex);
}

fn enable_cfg(key: &str, cfg: &str) {
    println!("cargo:rustc-cfg={}=\"{}\"", key, cfg);
}

fn rerun_on_env_change() {
    rerun_if_env_changes(NET_ENV_KEY);
    rerun_if_env_changes(DEX_ENV_KEY);
}

fn rerun_if_env_changes(env_key: &str) {
    println!("cargo:rerun-if-env-changed={}", env_key);
}
