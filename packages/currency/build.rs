fn main() {
    println!("cargo:rustc-cfg=net_name=\"{}\"", env!("NET_NAME"));
}
