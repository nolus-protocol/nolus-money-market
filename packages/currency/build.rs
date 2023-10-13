fn main() {
    println!("cargo:rustc-cfg=net_name=\"{}\"", env!("NET_NAME"));
    println!("cargo:rustc-cfg=dex=\"{}\"", env!("DEX"));
}
