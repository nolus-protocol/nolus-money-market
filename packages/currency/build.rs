fn main() {
    if let Some(net_name) = option_env!("NET_NAME") {
        println!("cargo:rustc-cfg=net_name=\"{net_name}\"");
    }
}
