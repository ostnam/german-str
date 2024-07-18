use german_str::GermanStr;
use proptest::proptest;

#[test]
#[cfg(target_pointer_width = "64")]
fn is_2_bytes() {
    assert_eq!(std::mem::size_of::<GermanStr>(), 16)
}

#[test]
fn test_new() {
    assert_eq!(
        GermanStr::new("hello world!").unwrap().as_str(),
        "hello world!",
    );
    assert_eq!(
        GermanStr::new("too long to fit on the stack").unwrap().as_str(),
        "too long to fit on the stack",
    );
}

#[test]
fn assert_traits() {
    fn f<T: Send + Sync + ::std::fmt::Debug + Clone>() {}
    f::<GermanStr>();
}

proptest! {
    #[test]
    fn conversion(src: String) {
        let german = GermanStr::new(&src).unwrap();
        let end = String::from(german);
        assert_eq!(src, end);
    }

    #[test]
    fn ordering(lhs: String, rhs: String) {
        let german_lhs = GermanStr::new(&lhs).unwrap();
        let german_rhs = GermanStr::new(&rhs).unwrap();
        assert_eq!(lhs.cmp(&rhs), german_lhs.cmp(&german_rhs));
    }

    #[test]
    fn equality(lhs: String, rhs: String) {
        let german_lhs = GermanStr::new(&lhs).unwrap();
        let german_rhs = GermanStr::new(&rhs).unwrap();
        assert_eq!(lhs == rhs, german_lhs == german_rhs);
    }
}
