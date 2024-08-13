use std::{fmt::Write, ops::Deref};

use assert_panic::assert_panic;
use proptest::proptest;

use german_str::{str_prefix, str_suffix, GermanStr, MAX_INLINE_BYTES, MAX_LEN};

#[test]
fn is_2_bytes() {
    assert_eq!(std::mem::size_of::<GermanStr>(), 16);
}

#[test]
fn assert_traits() {
    fn f<T: Send + Sync + ::std::fmt::Debug + Clone>() {}
    f::<GermanStr>();
}

#[test]
fn assert_largest_layout_valid() {
    assert!(std::alloc::Layout::array::<u8>(MAX_LEN).is_ok());
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
fn test_equality() {
    let a = GermanStr::new("aaaa").unwrap();
    let b = GermanStr::new("aaaab").unwrap();
    assert_ne!(a, b);
}

#[test]
fn test_default() {
    assert_eq!(
        GermanStr::default(),
        String::default(),
    );
}

proptest! {
    #[test]
    fn conversion(src: String) {
        let german = GermanStr::new(&src).unwrap();
        let end = String::from(german);
        assert_eq!(src, end);
    }

    #[test]
    fn deref(src: String) {
        let german = GermanStr::new(&src).unwrap();
        assert_eq!(&src, Deref::deref(&german));
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

    #[test]
    fn clone(val: String) {
        let german = GermanStr::new(&val).unwrap();
        assert_eq!(german, german.clone());
    }

    #[test]
    fn new_inline(val: String) {
        if val.len() > MAX_INLINE_BYTES {
            assert_panic!({
                GermanStr::new_inline(&val);
            });
        } else {
            let inline = GermanStr::new_inline(&val);
            assert_eq!(inline, val);
        }
    }

    #[test]
    fn prefix_bytes_slice(val: String) {
        let german = GermanStr::new(&val).unwrap();
        let prefix_len = val.len().min(4);
        assert_eq!(
            german.prefix_bytes_slice(),
            &val.as_bytes()[..prefix_len],
        );
    }

    #[test]
    fn prefix_bytes_array(val: String) {
        let german = GermanStr::new(&val).unwrap();
        let prefix_len = val.len().min(4);
        let mut og_array = [0; 4];
        og_array[..prefix_len].copy_from_slice(&val.as_bytes()[..prefix_len]);
        assert_eq!(
            german.prefix_bytes_array(),
            og_array,
        );
    }

    #[test]
    fn suffix_bytes_slice(val: String) {
        let german = GermanStr::new(&val).unwrap();
        assert_eq!(
            german.suffix_bytes_slice(),
            val.as_bytes().get(4..).unwrap_or_default(),
        );
    }

    #[test]
    fn test_as_str(val: String) {
        let german = GermanStr::new(&val).unwrap();
        assert_eq!(
            german.as_str(),
            &val,
        );
    }

    #[test]
    fn test_to_string(val: String) {
        let german = GermanStr::new(&val).unwrap();
        assert_eq!(
            &german.to_string(),
            &val,
        );
    }

    #[test]
    fn test_len(val: String) {
        let german = GermanStr::new(&val).unwrap();
        assert_eq!(
            german.len(),
            val.len(),
        );
    }

    #[test]
    fn test_is_empty(val: String) {
        let german = GermanStr::new(&val).unwrap();
        assert_eq!(
            german.is_empty(),
            val.is_empty(),
        );
    }

    #[test]
    fn test_debug(val: String) {
        let german = GermanStr::new(&val).unwrap();
        assert_eq!(
            format!("{german:?}"),
            format!("{val:?}"),
        );
    }

    #[test]
    fn test_display(val: String) {
        let german = GermanStr::new(&val).unwrap();
        assert_eq!(
            format!("{german}"),
            format!("{val}"),
        );
    }

    #[test]
    fn test_str_prefix(val: String) {
        let german = GermanStr::new(&val).unwrap();
        assert_eq!(
            str_prefix::<GermanStr>(german),
            str_prefix::<String>(val),
        );
    }

    #[test]
    fn test_str_suffix(val: String) {
        let german = GermanStr::new(&val).unwrap();
        assert_eq!(
            str_suffix::<GermanStr>(&german),
            str_suffix::<String>(&val),
        );
    }

    #[test]
    fn build_writer(values: Vec<String>) {
        let mut writer = german_str::Writer::new();
        let mut string = String::new();
        for val in &values {
            writer.write_str(val).unwrap();
            string.push_str(val);
        }
        let german = Into::<GermanStr>::into(writer);
        assert_eq!(german, string);
    }
}

#[cfg(feature = "serde")]
mod serde_tests {
    use std::collections::HashMap;
    use std::hash::Hash;

    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Serialize, Deserialize)]
    struct ExampleStruct<T: Eq + Hash> {
        raw: T,
        vec: Vec<T>,
        map: HashMap<T, T>,
    }

    proptest! {
        #[test]
        fn roundtrip(raw: String, vec: Vec<String>, map: HashMap<String, String>) {
            let initial = ExampleStruct { raw, vec, map };
            let json = serde_json::to_string(&initial).unwrap();
            let parsed = serde_json::from_str::<ExampleStruct<GermanStr>>(&json).unwrap();
            assert_eq!(parsed.raw, initial.raw);
            assert_eq!(parsed.vec, initial.vec);
            let mut parsed_vec = parsed
                .map
                .iter()
                .map(|(a, b)| (a.as_str(), b.as_str()))
                .collect::<Vec<_>>();
            parsed_vec.sort();

            let mut initial_vec = initial
                .map
                .iter()
                .map(|(a, b)| (a.as_str(), b.as_str()))
                .collect::<Vec<_>>();
            initial_vec.sort();
            assert_eq!(parsed_vec, initial_vec);
        }
    }
}
