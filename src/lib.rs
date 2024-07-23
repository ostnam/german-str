#![no_std]
#![cfg(target_pointer_width = "64")]

extern crate alloc;

use alloc::borrow::{Cow, ToOwned as _};
use alloc::boxed::Box;
use alloc::slice;
use alloc::string::String;
use alloc::sync::Arc;
use core::alloc::Layout;
use core::borrow::Borrow;
use core::ops::Deref;
use core::{cmp, ptr};
use core::str::FromStr;

/// The maximum number of chars a GermanStr can contain before requiring a heap
/// allocation.
pub const MAX_INLINE_BYTES: usize = 12;

/// The absolute maximum number of chars a GermanStr can hold.
/// Since the len is an u32, it is 2^32.
pub const MAX_LEN: usize = 2_usize.pow(32);

/// A string type with the following properties:
///
/// * Immutable.
/// * `size_of::<GermanStr>() == 16`
/// * Strings of 12 or less bytes are entirely located on the stack.
/// * Fast comparisons.
#[repr(C)]
pub struct GermanStr {
    /// Number of chars of the string.
    /// Serves as a tag for the variant used by the `last8` field, based on
    /// whether it is longer than `MAX_INLINE_BYTES` or not.
    len: u32,

    /// The first 4 bytes of the string. If it is shorter than 4 bytes, extra
    /// bytes are set to 0.
    ///
    /// Since an UTF-8 char can consist of 1-4 bytes, this field can store
    /// 1-4 chars, and potentially only part of the last char.
    /// In every case, this array can still be used to speed up comparisons
    /// because UTF-8 strings are ordered byte-wise.
    prefix: [u8; 4],

    /// If the string is longer than 12 bytes, is an owning, unique pointer to
    /// the chars on the heap.
    /// Otherwise, is an `[u8; 8]`, with extra bytes set to 0.
    /// The prefix is also included on the heap.
    last8: Last8,
}

#[derive(Copy, Clone)]
/// Holds the last 8 bytes of a `GermanStr`.
union Last8 {
    ptr: *const u8,

    /// If the string is shorter than 12 bytes, extra bytes are set to 0.
    buf: [u8; 8],
}

#[derive(Debug, Clone, Copy)]
/// Represents the reasons why creating a new `GermanStr` could fail.
pub enum InitError {
    /// `GermanStr`s use an u32 to store their length, hence they can't contain
    /// more than 2^32 bytes (~4GB).
    TooLong,
}

impl GermanStr {
    #[inline]
    /// Main function to create a GermanStr.
    pub fn new(src: impl AsRef<str>) -> Result<Self, InitError> {
        let src = src.as_ref();
        if src.len() > MAX_LEN {
            return Err(InitError::TooLong);
        }
        if src.len() <= MAX_INLINE_BYTES {
            return Ok(GermanStr::new_inline(src));
        }

        let layout = Layout::array::<u8>(src.len())
            .map_err(|_| InitError::TooLong)?;
        let ptr = unsafe {
            // Safety: layout is not zero-sized (src.len() <= MAX_INLINE_BYTES guard).
            alloc::alloc::alloc(layout)
        };
        if ptr.is_null() {
            alloc::alloc::handle_alloc_error(layout);
        }
        unsafe {
            // Safety:
            //   1. We assume src is a valid object.
            //   2. ptr is valid: it was checked for null and allocated
            //      for src.len() bytes.
            //   3. *_ u8 is always aligned.
            //   4. The 2 regions can't overlap since they belong to different objects.
            ptr::copy_nonoverlapping(src.as_bytes().as_ptr(), ptr, src.len());
        }
        Ok(GermanStr {
            len: src.len() as u32,
            prefix: str_prefix::<&str>(&src),
            last8: Last8 { ptr },
        })
    }

    #[inline]
    /// Attempts to create a GermanStr entirely stored in the struct itself,
    /// without heap allocations.
    ///
    /// Panics if `src.len()` > `MAX_INLINE_BYTES`.
    pub const fn new_inline(src: &str) -> GermanStr {
        assert!(src.len() <= MAX_INLINE_BYTES);

        let mut prefix = [0; 4];
        let mut i = 0;
        while i < src.len() && i < 4 {
            prefix[i] = src.as_bytes()[i];
            i += 1;
        }

        let mut buf = [0; 8];
        let mut i = 4;
        while i < src.len() && i < MAX_INLINE_BYTES {
            buf[i - 4] = src.as_bytes()[i];
            i += 1;
        }

        GermanStr {
            len: src.len() as u32,
            prefix,
            last8: Last8 { buf },
        }
    }

    #[inline]
    /// Safe accessor for last8.ptr.
    fn get_heap_ptr(&self) -> Option<*const u8> {
        if self.len as usize > MAX_INLINE_BYTES {
            Some(unsafe {
                self.last8.ptr
            })
        } else {
            None
        }
    }

    #[inline]
    /// Returns a slice containing the first 4 bytes of a `GermanStr`.
    /// Can be used for comparisons and ordering as is.
    /// Since an UTF-8 char can consist of 1-4 bytes, this slice can represent
    /// anywhere from 1 to 4 chars, and potentially only part of the last char.
    pub fn prefix_bytes_slice(&self) -> &[u8] {
        let prefix_len = self.len().min(4);
        &self.prefix[..prefix_len]
    }

    #[inline(always)]
    /// Returns an array containing the first 4 bytes of a `GermanStr`.
    /// If the string is shorter than 4 bytes, extra bytes are set to 0.
    /// Can be used for comparisons and ordering as is.
    /// Since an UTF-8 char can consist of 1-4 bytes, this array can represent
    /// anywhere from 1 to 4 chars, and potentially only part of the last char.
    pub const fn prefix_bytes_array(&self) -> [u8; 4] {
        self.prefix
    }

    #[inline]
    /// Returns a slice containing every byte of a `GermanStr`, except the first 4.
    pub fn suffix_bytes_slice(&self) -> &[u8] {
        let suffix_len = self.len().saturating_sub(4);
        if self.len as usize > MAX_INLINE_BYTES {
            unsafe {
                let ptr = self.last8.ptr;

                // Safety:
                // 1. The data is part of a single object.
                // 2. Pointer is checked for null at alloc.
                // 3. ptr has the correct offset for the length
                // 4. Heap values are properly initialized.
                // 5. Values in the slice are never mutated.
                slice::from_raw_parts(ptr.add(4), suffix_len)
            }
        } else {
            unsafe {
                &self.last8.buf[0..suffix_len]
            }
        }
    }

    #[inline(always)]
    pub fn as_str(&self) -> &str {
        Deref::deref(self)
    }

    #[allow(clippy::inherent_to_string_shadow_display)]
    #[inline(always)]
    pub fn to_string(&self) -> String {
        self.as_str().to_owned()
    }

    #[inline(always)]
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    #[inline(always)]
    pub const fn is_empty(&self) -> bool  {
        self.len == 0
    }

    #[inline(always)]
    pub const fn is_heap_allocated(&self) -> bool {
        self.len as usize > MAX_INLINE_BYTES
    }
}

impl Clone for GermanStr {
    #[inline]
    fn clone(&self) -> Self {
        if let Some(self_ptr) = self.get_heap_ptr() {
            let (ptr, layout) = unsafe {
                // Safety: If len was too high for this layout, we couldn't
                // have made self in the first place.
                let layout = Layout::array::<u8>(self.len()).unwrap_unchecked();

                // Safety: layout is not zero-sized, otherwise we would store the string inplace.
                let ptr = alloc::alloc::alloc(layout);
                (ptr, layout)
            };
            if ptr.is_null() {
                alloc::alloc::handle_alloc_error(layout);
            }
            unsafe {
                // Safety:
                //   1. Both pointers are valid.
                //   2. *_ u8 is always aligned.
                //   3. The 2 regions can't overlap since they belong to different objects.
                ptr::copy_nonoverlapping(self_ptr, ptr, self.len());
            }
            GermanStr {
                prefix: self.prefix,
                len: self.len,
                last8: Last8 { ptr },
            }
        } else {
            GermanStr {
                len: self.len,
                prefix: self.prefix,
                last8: self.last8,
            }
        }
    }
}

impl Drop for GermanStr {
    #[inline]
    fn drop(&mut self) {
        if let Some(ptr) = self.get_heap_ptr() {
            let ptr = ptr.cast_mut();
            unsafe {
                // Safety: this call can only fail if self.len is too long.
                // We can only create a long `GermanStr` using GermanStr::new: if `self.len`
                // was too long, we'd get an error when we try to create the GermanStr.
                let layout = Layout::array::<u8>(self.len as usize).unwrap_unchecked();
                alloc::alloc::dealloc(ptr, layout);
            }
        }
        // In the case where len <= MAX_INLINE_BYTES, no heap data is owned and
        // no deallocation is needed.
    }
}

// Safety: According to the rustonomicon, "something can safely be Send unless it shares mutable
// state with something else without enforcing exclusive access to it."
// The `ptr` is never shared between `GermanStr`, so there's no shared mutable state.
unsafe impl Send for GermanStr {}

// Safety: Again, according to the rustonomicon, there's no issue here since GermanStr are
// immutable.
unsafe impl Sync for GermanStr {}

impl Deref for GermanStr {
    type Target = str;

    #[inline(always)]
    fn deref(&self) -> &str {
        let len = self.len as usize;
        if len <= MAX_INLINE_BYTES {
            let prefix_ptr: *const [u8; 4] = &self.prefix;
            unsafe {
                let slice = slice::from_raw_parts(prefix_ptr.cast(), len);
                core::str::from_utf8_unchecked(slice)
            }
        } else {
            unsafe {
                let ptr = self.last8.ptr;
                let slice = slice::from_raw_parts(ptr, len);
                core::str::from_utf8_unchecked(slice)
            }
        }
    }
}

impl AsRef<str> for GermanStr {
    #[inline(always)]
    fn as_ref(&self) -> &str {
        Deref::deref(self)
    }
}

impl PartialEq<GermanStr> for GermanStr {
    #[inline(always)]
    fn eq(&self, other: &GermanStr) -> bool {
        self.prefix == other.prefix && self.suffix_bytes_slice() == other.suffix_bytes_slice()
    }
}
impl Eq for GermanStr {}

impl Ord for GermanStr {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.prefix.cmp(&other.prefix)
            .then_with(
                || self.suffix_bytes_slice().cmp(other.suffix_bytes_slice())
            )
    }
}

impl Default for GermanStr {
    #[inline(always)]
    fn default() -> GermanStr {
        GermanStr {
            len: 0,
            prefix: [0; 4],
            last8: Last8 { buf: [0; 8] },
        }
    }
}

impl FromStr for GermanStr {
    type Err = InitError;

    #[inline]
    fn from_str(s: &str) -> Result<GermanStr, Self::Err> {
        GermanStr::new(s)
    }
}

impl Borrow<str> for GermanStr {
    #[inline(always)]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl core::fmt::Debug for GermanStr {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        core::fmt::Debug::fmt(self.as_str(), f)
    }
}


impl core::fmt::Display for GermanStr {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        core::fmt::Display::fmt(self.as_str(), f)
    }
}

impl core::fmt::Display for InitError {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        core::fmt::Display::fmt(
            match self {
                InitError::TooLong => "Tried to initialize a GermanStr longer than 4GB.",
            },
            f
        )
    }
}

impl core::hash::Hash for GermanStr {
    #[inline(always)]
    fn hash<H: core::hash::Hasher>(&self, hasher: &mut H) {
        self.as_str().hash(hasher);
    }
}


impl PartialOrd for GermanStr {
    #[inline(always)]
    fn partial_cmp(&self, other: &GermanStr) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq<str> for GermanStr {
    #[inline(always)]
    fn eq(&self, other: &str) -> bool {
        self.prefix == str_prefix::<&str>(other) &&
        self.suffix_bytes_slice() == str_suffix::<&str>(&other)
    }
}

impl PartialEq<GermanStr> for str {
    #[inline(always)]
    fn eq(&self, other: &GermanStr) -> bool {
        other.prefix == str_prefix::<&str>(self) &&
        other.suffix_bytes_slice() == str_suffix::<&str>(&self)
    }
}

impl<'a> PartialEq<&'a str> for GermanStr {
    #[inline(always)]
    fn eq(&self, other: &&'a str) -> bool {
        self.prefix == str_prefix::<&str>(other) &&
        self.suffix_bytes_slice() == str_suffix::<&str>(&other)
    }
}

impl<'a> PartialEq<GermanStr> for &'a str {
    #[inline(always)]
    fn eq(&self, other: &GermanStr) -> bool {
        other.prefix == str_prefix::<&str>(self) &&
        other.suffix_bytes_slice() == str_suffix::<&str>(&self)
    }
}

impl PartialEq<String> for GermanStr {
    #[inline(always)]
    fn eq(&self, other: &String) -> bool {
        self.prefix == str_prefix::<&str>(other) &&
        self.suffix_bytes_slice() == str_suffix::<&str>(&other)
    }
}

impl PartialEq<GermanStr> for String {
    #[inline(always)]
    fn eq(&self, other: &GermanStr) -> bool {
        other.prefix == str_prefix::<&str>(self) &&
        other.suffix_bytes_slice() == str_suffix::<&str>(&self)
    }
}

impl<'a> PartialEq<&'a String> for GermanStr {
    #[inline(always)]
    fn eq(&self, other: &&'a String) -> bool {
        self.prefix == str_prefix::<&str>(other) &&
        self.suffix_bytes_slice() == str_suffix::<&str>(&other)
    }
}

impl<'a> PartialEq<GermanStr> for &'a String {
    #[inline(always)]
    fn eq(&self, other: &GermanStr) -> bool {
        other.prefix == str_prefix::<&str>(self) &&
        other.suffix_bytes_slice() == str_suffix::<&str>(&self)
    }
}

impl TryFrom<&str> for GermanStr {
    type Error = InitError;

    #[inline]
    fn try_from(s: &str) -> Result<GermanStr, InitError> {
        GermanStr::new(s)
        }
    }

impl TryFrom<&mut str> for GermanStr {
    type Error = InitError;

    #[inline]
    fn try_from(s: &mut str) -> Result<GermanStr,InitError> {
        GermanStr::new(s)
    }
}

impl TryFrom<&String> for GermanStr {
    type Error = InitError;

    #[inline]
    fn try_from(s: &String) -> Result<GermanStr,InitError> {
        GermanStr::new(s)
    }
}

impl TryFrom<String> for GermanStr {
    type Error = InitError;

    #[inline(always)]
    fn try_from(text: String) -> Result<Self, Self::Error> {
        Self::new(text)
    }
}

impl TryFrom<Box<str>> for GermanStr {
    type Error = InitError;

    #[inline(always)]
    fn try_from(s: Box<str>) -> Result<GermanStr, Self::Error> {
        GermanStr::new(s)
    }
}

impl TryFrom<Arc<str>> for GermanStr {
    type Error = InitError;

    #[inline(always)]
    fn try_from(s: Arc<str>) -> Result<GermanStr, Self::Error> {
        GermanStr::new(s)
    }
}

impl From<GermanStr> for Arc<str> {
    #[inline(always)]
    fn from(text: GermanStr) -> Self {
        text.as_str().into()
    }
}

impl<'a> TryFrom<Cow<'a, str>> for GermanStr {
    type Error = InitError;

    #[inline]
    fn try_from(s: Cow<'a, str>) -> Result<GermanStr,InitError> {
        GermanStr::new(s)
    }
}

impl From<GermanStr> for String {
    #[inline(always)]
    fn from(text: GermanStr) -> Self {
        text.as_str().into()
    }
}

#[inline]
/// Returns the first 4 bytes of a string.
/// If the string has less than 4 bytes, extra bytes are set to 0.
pub fn str_prefix<T>(src: impl AsRef<str>) -> [u8; 4] {
    let src_bytes = src.as_ref().as_bytes();
    let prefix_len = src_bytes.len().min(4);
    let mut bytes = [0; 4];
    bytes[..prefix_len].copy_from_slice(&src_bytes[..prefix_len]);
    bytes
}

#[inline]
/// Returns a slice to every byte of a string, skipping the first 4.
pub fn str_suffix<T>(src: &impl AsRef<str>) -> &[u8] {
    src.as_ref().as_bytes().get(4..).unwrap_or_default()
}

#[cfg(feature = "serde")]
mod serde {
    use alloc::string::String;
    use alloc::vec::Vec;
    use core::fmt;

    use serde::de::{Deserializer, Error, Unexpected, Visitor};

    use crate::GermanStr;

    fn deserialize<'de: 'a, 'a, D>(deserializer: D) -> Result<GermanStr, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct GermanStrVisitor;

        impl<'a> Visitor<'a> for GermanStrVisitor {
            type Value = GermanStr;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                GermanStr::new(v).map_err(Error::custom)
            }

            fn visit_borrowed_str<E>(self, v: &'a str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                GermanStr::new(v).map_err(Error::custom)
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: Error,
            {
                GermanStr::new(v).map_err(Error::custom)
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: Error,
            {
                match core::str::from_utf8(v) {
                    Ok(s) => GermanStr::new(s).map_err(Error::custom),
                    Err(_) => Err(Error::invalid_value(Unexpected::Bytes(v), &self)),
                }
            }

            fn visit_borrowed_bytes<E>(self, v: &'a [u8]) -> Result<Self::Value, E>
            where
                E: Error,
            {
                match core::str::from_utf8(v) {
                    Ok(s) => GermanStr::new(s).map_err(Error::custom),
                    Err(_) => Err(Error::invalid_value(Unexpected::Bytes(v), &self)),
                }
            }

            fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
            where
                E: Error,
            {
                match String::from_utf8(v) {
                    Ok(s) => GermanStr::new(s).map_err(Error::custom),
                    Err(e) => Err(Error::invalid_value(
                        Unexpected::Bytes(&e.into_bytes()),
                        &self,
                    )),
                }
            }
        }

        deserializer.deserialize_str(GermanStrVisitor)
    }

    impl serde::Serialize for GermanStr {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            self.as_str().serialize(serializer)
        }
    }

    impl<'de> serde::Deserialize<'de> for GermanStr {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserialize(deserializer)
        }
    }
}
