fn main() {
    if option_env!("LOCAL_NET_ADMIN_CONTRACT")
        .map_or(false, |value| ["1", "y", "Y"].contains(&value))
    {
        println!("cargo:rustc-cfg=feature=\"admin_contract_exec\"");
    }
}
