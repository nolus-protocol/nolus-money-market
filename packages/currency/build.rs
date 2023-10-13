fn main() {
    println!("cargo:rustc-cfg=net=\"{}\"", env!("NET"));
    println!("cargo:rustc-cfg=dex=\"{}\"", env!("DEX"));
}
