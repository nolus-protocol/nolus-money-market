use currencies::{
    testing::{PaymentC1, PaymentC3, PaymentC4, PaymentC5, PaymentC6, PaymentC7, PaymentC9},
    Lpn as BaseCurrency, PaymentGroup as PriceCurrencies,
};
use currency::CurrencyDef;
use tree::HumanReadableTree;

use crate::api::swap::SwapTarget;

pub fn dummy_swap_tree() -> HumanReadableTree<SwapTarget<PriceCurrencies>> {
    sdk::cosmwasm_std::from_json(format!(
        r#"{{
            "value":[0,"{lpn}"],
            "children":[
                {{
                    "value":[3,"{p4}"],
                    "children":[
                        {{
                            "value":[2,"{p5}"],
                            "children":[
                                {{"value":[1,"{p3}"]}}
                            ]
                        }},
                        {{"value":[15,"{p6}"]}}
                    ]
                }},
                {{
                    "value":[4,"{p1}"],
                    "children":[
                        {{"value":[5,"{p7}"]}}
                    ]
                }},
                {{
                    "value":[4,"{p9}"]
                }}
            ]
        }}"#,
        lpn = BaseCurrency::definition().dto(),
        p4 = PaymentC4::definition().dto(),
        p5 = PaymentC5::definition().dto(),
        p3 = PaymentC3::definition().dto(),
        p1 = PaymentC1::definition().dto(),
        p6 = PaymentC6::definition().dto(),
        p7 = PaymentC7::definition().dto(),
        p9 = PaymentC9::definition().dto(),
    ))
    .expect("The dummy swap tree is valid")
}

pub fn minimal_swap_tree() -> HumanReadableTree<SwapTarget<PriceCurrencies>> {
    sdk::cosmwasm_std::from_json(format!(
        r#"{{
            "value":[0,"{lpn}"],
            "children":[
                {{
                    "value":[1,"{p9}"]
                }}
            ]
        }}"#,
        lpn = BaseCurrency::definition().dto(),
        p9 = PaymentC9::definition().dto(),
    ))
    .expect("The dummy swap tree is valid")
}

pub fn invalid_pair_swap_tree() -> HumanReadableTree<SwapTarget<PriceCurrencies>> {
    sdk::cosmwasm_std::from_json(format!(
        r#"{{
            "value":[0,"{lpn}"],
            "children":[
                {{
                    "value":[1,"{p5}"]
                }}
            ]
        }}"#,
        lpn = BaseCurrency::definition().dto(),
        p5 = PaymentC5::definition().dto(),
    ))
    .expect("The dummy swap tree is valid")
}
