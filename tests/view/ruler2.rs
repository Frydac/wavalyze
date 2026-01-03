#[test]
fn test_format_compact() {
    assert_eq!(wavalyze::view::ruler2::format_compact(123456), "3,456");
}
