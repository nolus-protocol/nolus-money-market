#[macro_export]
macro_rules! swap_tree {
    ($base_currency: expr, $(($id: literal, $currency: expr $(,)?)),+ $(,)?) => {
        serde_json_wasm::from_str::<::tree::HumanReadableTree<::swap::SwapTarget>>(&::tree::tree_json! {
            value: format!("[0, {:?}]", $base_currency),
            children: [
                $({ value: format!("[{}, {:?}]", $id, $currency) }),+
            ],
        })
            .unwrap()
    };
}
