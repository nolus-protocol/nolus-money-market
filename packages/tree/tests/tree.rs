use serde_json_wasm::from_str;

use tree::Tree;

mod serde {
    use super::*;

    #[test]
    fn only_root() {
        let tree: Tree<u32> = from_str(r#"{"nodes":[{"parent":0,"value":5}]}"#).unwrap();

        assert_eq!(*tree.root().unwrap().value(), 5);
    }

    #[test]
    fn invalid_root() {
        assert!(
            from_str::<Tree<u32>>(r#"{"nodes":[{"parent":1,"value":5}]}"#)
                .ok()
                .is_none()
        );
    }

    #[test]
    fn with_2_levels() {
        let tree: Tree<u32> = from_str(r#"{"nodes":[{"parent":0,"value":5},{"parent":0,"value":4},{"parent":0,"value":3},{"parent":0,"value":6}]}"#).unwrap();

        assert_eq!(*tree.root().unwrap().value(), 5);

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
        let tree: Tree<u32> = from_str(r#"{"nodes":[{"parent":0,"value":5},{"parent":0,"value":4},{"parent":1,"value":6},{"parent":0,"value":3},{"parent":1,"value":7}]}"#).unwrap();

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
    fn with_3_levels_forward_index() {
        assert!(from_str::<Tree<u32>>(r#"{"nodes":[{"parent":0,"value":5},{"parent":2,"value":4},{"parent":0,"value":3},{"parent":2,"value":6}]}"#).ok().is_none());
    }
}
