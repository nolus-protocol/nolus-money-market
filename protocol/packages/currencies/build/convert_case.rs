use std::iter;

pub(super) fn snake_case_to_upper_camel_case(mut input: &str) -> String {
    iter::from_fn(move || {
        input
            .find('_')
            .or_else(|| (!input.is_empty()).then_some(input.len()))
            .map(|index| {
                let substring = &input[..index];

                input = input.get(index + 1..).unwrap_or("");

                substring
            })
    })
    .flat_map(|substring| {
        let mut chars = substring.chars();

        chars
            .next()
            .map(|first_character| first_character.to_ascii_uppercase())
            .into_iter()
            .chain(chars.map(|ch| ch.to_ascii_lowercase()))
    })
    .collect()
}

#[test]
fn test() {
    assert_eq!(snake_case_to_upper_camel_case("aa_b2_cc_3_4d"), "AaB2Cc34d");
}
