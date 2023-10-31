use ::sdk::cosmwasm_std;

use super::{HumanReadableTree, Tree};

#[test]
fn compare_with_human_readable() {
    let tree: Tree<u32> = cosmwasm_std::from_json(r#"[{"parent":0,"value":5},{"parent":0,"value":4},{"parent":1,"value":6},{"parent":1,"value":7},{"parent":0,"value":3}]"#).unwrap();

    let human_readable: HumanReadableTree<u32> = cosmwasm_std::from_json(
        r#"{"value":5,"children":[{"value":4,"children":[{"value":6},{"value":7}]},{"value":3}]}"#,
    )
    .unwrap();

    assert_eq!(tree, human_readable.into_tree());
}

#[test]
#[should_panic = "Trees are not equal"]
/// This test should fail because while both trees are logically the same they produce different vectors.
fn compare_with_human_readable_failing() {
    let tree: Tree<u32> = cosmwasm_std::from_json(r#"[{"parent":0,"value":5},{"parent":0,"value":4},{"parent":1,"value":6},{"parent":0,"value":3},{"parent":1,"value":7}]"#).unwrap();

    let human_readable: HumanReadableTree<u32> = cosmwasm_std::from_json(
        r#"{"value":5,"children":[{"value":4,"children":[{"value":6},{"value":7}]},{"value":3}]}"#,
    )
    .unwrap();

    assert_eq!(tree, human_readable.into_tree(), "Trees are not equal");
}
