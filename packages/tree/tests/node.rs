use tree::Node;

#[test]
fn preserve_ordering() {
    const ROOT: u32 = 0;
    const LEFT: u32 = 2;
    const MIDDLE: u32 = 1;
    const RIGHT: u32 = 3;

    let mut node = Node::new(ROOT);

    node.extend([Node::new(LEFT), Node::new(MIDDLE)]);

    node.extend([RIGHT]);

    assert_eq!(node.value(), &ROOT);
    assert!(node
        .children()
        .iter()
        .zip([LEFT, MIDDLE, RIGHT])
        .all(|(node, expected)| *node.value() == expected));
}
