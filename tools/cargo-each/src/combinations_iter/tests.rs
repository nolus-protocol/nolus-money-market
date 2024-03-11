#[test]
fn cross_join() {
    let mut result: Vec<String> = super::cross_join(
        vec!["1".into(), "2".into()].into_iter(),
        ["a", "b"].into_iter(),
    )
    .collect();

    result.sort();

    assert_eq!(
        result,
        ["1,a".to_string(), "1,b".into(), "2,a".into(), "2,b".into()]
    );
}

#[test]
fn cross_join_with_empty() {
    let mut result: Vec<String> = super::cross_join(
        vec![String::new(), "2".into()].into_iter(),
        ["a", ""].into_iter(),
    )
    .collect();

    result.sort();

    assert_eq!(
        result,
        [String::new(), "2".into(), "2,a".into(), "a".into()]
    );
}

#[test]
fn build_combinations() {
    let mut output: Vec<String> =
        super::build_combinations(["1", "2", "3", "4", "5"].into_iter()).collect();

    output.sort();

    assert_eq!(
        output,
        [
            "".to_string(),
            "1".into(),
            "1,2".into(),
            "1,2,3".into(),
            "1,2,3,4".into(),
            "1,2,3,4,5".into(),
            "1,2,3,5".into(),
            "1,2,4".into(),
            "1,2,4,5".into(),
            "1,2,5".into(),
            "1,3".into(),
            "1,3,4".into(),
            "1,3,4,5".into(),
            "1,3,5".into(),
            "1,4".into(),
            "1,4,5".into(),
            "1,5".into(),
            "2".into(),
            "2,3".into(),
            "2,3,4".into(),
            "2,3,4,5".into(),
            "2,3,5".into(),
            "2,4".into(),
            "2,4,5".into(),
            "2,5".into(),
            "3".into(),
            "3,4".into(),
            "3,4,5".into(),
            "3,5".into(),
            "4".into(),
            "4,5".into(),
            "5".into(),
        ]
    );
}

#[test]
fn build_combinations_with_at_least_one() {
    let mut output: Vec<String> =
        super::build_combinations_with_at_least_one(["1", "2", "3", "4", "5"].into_iter())
            .collect();

    output.sort();

    assert_eq!(
        output,
        [
            "1".to_string(),
            "1,2".into(),
            "1,2,3".into(),
            "1,2,3,4".into(),
            "1,2,3,4,5".into(),
            "1,2,3,5".into(),
            "1,2,4".into(),
            "1,2,4,5".into(),
            "1,2,5".into(),
            "1,3".into(),
            "1,3,4".into(),
            "1,3,4,5".into(),
            "1,3,5".into(),
            "1,4".into(),
            "1,4,5".into(),
            "1,5".into(),
            "2".into(),
            "2,3".into(),
            "2,3,4".into(),
            "2,3,4,5".into(),
            "2,3,5".into(),
            "2,4".into(),
            "2,4,5".into(),
            "2,5".into(),
            "3".into(),
            "3,4".into(),
            "3,4,5".into(),
            "3,5".into(),
            "4".into(),
            "4,5".into(),
            "5".into(),
        ]
    );
}
