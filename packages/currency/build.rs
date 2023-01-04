fn main() {
    if option_env!("ALT_NET_SYMBOLS").map_or(false, |value| ["1", "y", "Y"].contains(&value)) {
        println!("cargo:rustc-cfg=feature=\"alt_net_symbols\"");
    }
}
