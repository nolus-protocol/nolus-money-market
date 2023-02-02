#[macro_export]
macro_rules! swap_tree {
    ($(($id: literal, $currency: expr $(,)?)),+ $(,)?) => {
        serde_json_wasm::from_str(&tree::tree_json! {
            value: format!("[0, {:?}]", Usdc::TICKER),
            children: [
                $({ value: format!("[{}, {:?}]", $id, $currency) }),+
            ],
        })
            .unwrap()
    };
}
