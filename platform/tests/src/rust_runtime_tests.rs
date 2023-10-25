#[test]
#[should_panic]
#[allow(arithmetic_overflow)]
fn overflow_panic() {
    let _ = u8::MAX + 1_u8;
}
