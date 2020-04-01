#[test]
fn find_a_match() {
    let mut res = Vec::new();
    find_matches("lorem ipsum\ndolor sit amet", "lorem", &mut res);
    assert_eq!(res, b"lorem ipsum");
}
