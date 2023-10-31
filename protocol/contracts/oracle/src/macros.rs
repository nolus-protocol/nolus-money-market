#[macro_export]
macro_rules! swap_tree {
    ({base: $base_currency: expr}, $(($id: literal, $currency: expr $(,)?)),+ $(,)?) => {
        sdk::cosmwasm_std::from_json::<::tree::HumanReadableTree<::swap::SwapTarget>>(&::tree::tree_json! {
            value: format!("[0, {:?}]", $base_currency),
            children: [
                $({ value: format!("[{}, {:?}]", $id, $currency) }),+
            ],
        })
            .unwrap()
    };
}
