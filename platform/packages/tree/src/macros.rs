// TODO [Edition=2024] Remove `#[expect]`.
#[cfg(any(feature = "testing", test))]
#[expect(edition_2024_expr_fragment_specifier)]
#[macro_export]
macro_rules! tree_json {
    (value: $value: expr $(,)?) => {
        format!(
            r#"{{"value":{value}}}"#,
            value = $value,
        )
    };
    ((raw) value: $value: expr $(,)?) => {
        $crate::tree_json! { value: ::core::stringify!($value) }
    };
    (value: $value: expr, children: [$({$($children:tt)+}),+ $(,)?] $(,)?) => {
        format!(
            r#"{{"value":{value},"children":[{children}]}}"#,
            value = $value,
            children = [
                $(
                    $crate::tree_json! {
                        $($children)+
                    }
                ),+
            ].join(","),
        )
    };
    ((raw) value: $value: expr, children: [$({$($children:tt)+}),+ $(,)?] $(,)?) => {
        $crate::tree_json! { value: ::core::stringify!($value), children: [$({$($children)+}),+] }
    }
}

#[cfg(test)]
#[test]
fn test_macro() {
    assert_eq!(
        tree_json! {
            (raw) value: "1",
            children: [
                { value: 1 },
                { (raw) value: [2] }
            ]
        },
        r#"{"value":"1","children":[{"value":1},{"value":[2]}]}"#
    );
}
