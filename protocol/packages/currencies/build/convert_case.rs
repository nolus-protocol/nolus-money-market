use std::iter;

pub(super) fn snake_case_to_upper_camel_case(mut input: &str) -> String {
    let mut string = String::new();

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
    .for_each(|substring| {
        let mut chars = substring.chars();

        if let Some(first_character) = chars.next() {
            string.push(first_character.to_ascii_uppercase());

            chars
                .map(|ch| ch.to_ascii_lowercase())
                .for_each(|ch| string.push(ch));
        }
    });

    string
}

#[test]
fn test() {
    assert_eq!(snake_case_to_upper_camel_case("aa_b2_cc_3_4d"), "AaB2Cc34d");
}
