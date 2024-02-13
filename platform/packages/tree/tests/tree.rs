use sdk::cosmwasm_std;
use tree::{FindBy, HumanReadableTree, Tree};

mod serde {
    use super::*;

    #[test]
    fn only_root() {
        let tree: Tree<u32> = cosmwasm_std::from_json(r#"[{"parent":0,"value":5}]"#).unwrap();

        assert_eq!(*tree.root().value(), 5);
    }

    #[test]
    fn with_2_levels() {
        let tree: Tree<u32> = cosmwasm_std::from_json(r#"[{"parent":0,"value":5},{"parent":0,"value":4},{"parent":0,"value":3},{"parent":0,"value":6}]"#).unwrap();

        assert_eq!(*tree.root().value(), 5);

        for expected_value in 3..=6 {
            assert_eq!(
                *tree
                    .find_by(move |&value| value == expected_value)
                    .unwrap()
                    .value(),
                expected_value
            );
        }
    }

    #[test]
    fn with_3_levels() {
        let tree: Tree<u32> = cosmwasm_std::from_json(r#"[{"parent":0,"value":5},{"parent":0,"value":4},{"parent":1,"value":6},{"parent":0,"value":3},{"parent":1,"value":7}]"#).unwrap();

        for (parent_value, expected_value) in [
            (None, 5),
            (Some(5), 4),
            (Some(4), 6),
            (Some(5), 3),
            (Some(4), 7),
        ] {
            let node = tree.find_by(move |&value| value == expected_value).unwrap();

            assert_eq!(node.parent().map(|parent| *parent.value()), parent_value);
            assert_eq!(*node.value(), expected_value);
        }
    }

    #[test]
    fn human_readable() {
        let original: HumanReadableTree<u32> = cosmwasm_std::from_json(
            r#"{"value":5,"children":[{"value":4,"children":[{"value":6},{"value":7}]},{"value":3}]}"#,
        )
            .unwrap();

        let transformed_back: HumanReadableTree<u32> =
            original.clone().into_tree().into_human_readable();

        assert_eq!(transformed_back, original);
    }

    #[test]
    fn parent_iter() {
        let tree: Tree<u32> = cosmwasm_std::from_json::<HumanReadableTree<u32>>(
            r#"{"value":5,"children":[{"value":4,"children":[{"value":6},{"value":7}]},{"value":3}]}"#,
        )
            .unwrap()
            .into_tree();

        assert_eq!(
            tree.find_by(|&v| v == 4)
                .unwrap()
                .parents_iter()
                .map(|node| *node.value())
                .collect::<Vec<u32>>(),
            vec![5]
        );

        assert_eq!(
            tree.find_by(|&v| v == 4)
                .unwrap()
                .parents_iter()
                .rev()
                .map(|node| *node.value())
                .collect::<Vec<u32>>(),
            vec![5]
        );

        assert_eq!(
            tree.find_by(|&v| v == 7)
                .unwrap()
                .parents_iter()
                .map(|node| *node.value())
                .collect::<Vec<u32>>(),
            vec![4, 5]
        );

        assert_eq!(
            tree.find_by(|&v| v == 7)
                .unwrap()
                .parents_iter()
                .rev()
                .map(|node| *node.value())
                .collect::<Vec<u32>>(),
            vec![5, 4]
        );
    }
}
