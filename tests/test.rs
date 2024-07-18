use german_str::GermanStr;

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
