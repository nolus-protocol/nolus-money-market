use std::fmt::Debug;

use serde::Deserialize;

use tree::{HumanReadableTree, Tree};

const CORRECT_TREE_JSON: &str = r#"{
    "root": 0,
    "parent_indexes": [0, 1, 2, 1, 4, 4, 6, 7, 4, 0, 10],
    "branches_and_leafs": [2, 1, 4, 3, 6, 5, 8, 7, 10, 9, 12]
}"#;

fn deserialize_tree<'r, T>(json: &'r str) -> Tree<T>
where
    T: Deserialize<'r>,
{
    serde_json::from_str(json).unwrap()
}

#[test]
fn check_correct_tree() {
    let tree: Tree<u32> = deserialize_tree(CORRECT_TREE_JSON);

    tree.check_tree();
}

#[test]
#[should_panic = "Nodes can only be defined directly under the node they belong to! Expected: 3, got: 2"]
fn check_incorrect_tree_indirect() {
    const TREE_JSON: &str = r#"{
        "root": 0,
        "parent_indexes": [0, 1, 1, 2],
        "branches_and_leafs": [1, 2, 3, 4]
    }"#;

    let tree: Tree<u32> = deserialize_tree(TREE_JSON);

    tree.check_tree();
}

#[test]
#[should_panic = "Nodes can only belong to nodes on the left-side of them! Expected: <=0, got: 1"]
fn check_incorrect_tree_first_belongs_to_self() {
    const TREE_JSON: &str = r#"{
        "root": 0,
        "parent_indexes": [1],
        "branches_and_leafs": [1]
    }"#;

    let tree: Tree<u32> = deserialize_tree(TREE_JSON);

    tree.check_tree();
}

#[test]
#[should_panic = "Nodes can only belong to nodes on the left-side of them! Expected: <=1, got: 2"]
fn check_incorrect_tree_second_belongs_to_self() {
    const TREE_JSON: &str = r#"{
        "root": 0,
        "parent_indexes": [0, 2],
        "branches_and_leafs": [1, 2]
    }"#;

    let tree: Tree<u32> = deserialize_tree(TREE_JSON);

    tree.check_tree();
}

#[test]
#[should_panic = "Nodes can only belong to nodes on the left-side of them! Expected: <=0, got: 2"]
fn check_incorrect_tree_belongs_to_undefined_right() {
    const TREE_JSON: &str = r#"{
        "root": 0,
        "parent_indexes": [2],
        "branches_and_leafs": [1]
    }"#;

    let tree: Tree<u32> = deserialize_tree(TREE_JSON);

    tree.check_tree();
}

#[test]
#[should_panic = "Nodes can only belong to nodes on the left-side of them! Expected: <=0, got: 2"]
fn check_incorrect_tree_belongs_to_right() {
    const TREE_JSON: &str = r#"{
        "root": 0,
        "parent_indexes": [2, 0],
        "branches_and_leafs": [1, 2]
    }"#;

    let tree: Tree<u32> = deserialize_tree(TREE_JSON);

    tree.check_tree();
}

#[test]
fn root_direct_children() {
    let tree: Tree<u32> = deserialize_tree(CORRECT_TREE_JSON);

    let collected_via_nodes_iter = tree
        .direct_children()
        .map(|node| *node.value())
        .collect::<Vec<_>>();

    assert_eq!(collected_via_nodes_iter, &[2, 9]);
}

#[test]
fn childs_direct_children() {
    let tree: Tree<u32> = deserialize_tree(CORRECT_TREE_JSON);

    let collected_via_nodes_iter = tree
        .direct_children()
        .nth(0)
        .unwrap()
        .direct_children()
        .map(|node| *node.value())
        .collect::<Vec<_>>();

    assert_eq!(collected_via_nodes_iter, &[1, 3]);
}

#[test]
fn root_depth_first_iters() {
    let tree: Tree<u32> = deserialize_tree(CORRECT_TREE_JSON);

    let collected_via_nodes_iter = tree
        .depth_first_nodes_iter()
        .map(|node| *node.value())
        .collect::<Vec<_>>();

    assert_eq!(
        collected_via_nodes_iter,
        tree.depth_first_values_iter().copied().collect::<Vec<_>>()
    );

    assert_eq!(
        collected_via_nodes_iter,
        &[0, 2, 1, 4, 3, 6, 5, 8, 7, 10, 9, 12]
    );
}

#[test]
fn child_node_depth_first_iters() {
    fn test_fn<T>(tree: &Tree<T>, nth_direct_child: usize, expected_values: &[T])
    where
        T: Debug + Copy + Eq,
    {
        let node = tree.direct_children().nth(nth_direct_child).unwrap();

        let collected_via_nodes_iter = node
            .depth_first_nodes_iter()
            .map(|node| *node.value())
            .collect::<Vec<_>>();

        assert_eq!(
            collected_via_nodes_iter,
            node.depth_first_values_iter().copied().collect::<Vec<_>>()
        );

        assert_eq!(collected_via_nodes_iter, expected_values);
    }

    let tree: Tree<u32> = deserialize_tree(CORRECT_TREE_JSON);

    test_fn(&tree, 0, &[2, 1, 4, 3, 6, 5, 8, 7, 10]);

    test_fn(&tree, 1, &[9, 12]);
}

#[test]
fn from_human_readable_tree() {
    const EXPECTED_TREE: &str = r#"{
        "root": 0,
        "parent_indexes": [0, 1, 2, 1, 0, 5, 6],
        "branches_and_leafs": [1, 2, 3, 4, 5, 6, 7]
    }"#;

    let tree: HumanReadableTree<u32> = HumanReadableTree::Branch {
        value: 0,
        children: vec![
            HumanReadableTree::Branch {
                value: 1,
                children: vec![
                    HumanReadableTree::Branch {
                        value: 2,
                        children: vec![HumanReadableTree::Leaf { value: 3 }].into_boxed_slice(),
                    },
                    HumanReadableTree::Leaf { value: 4 },
                ]
                .into_boxed_slice(),
            },
            HumanReadableTree::Branch {
                value: 5,
                children: vec![HumanReadableTree::Branch {
                    value: 6,
                    children: vec![HumanReadableTree::Leaf { value: 7 }].into_boxed_slice(),
                }]
                .into_boxed_slice(),
            },
        ]
        .into_boxed_slice(),
    };

    assert_eq!(
        Tree::try_from(tree).unwrap(),
        deserialize_tree(EXPECTED_TREE)
    );
}
