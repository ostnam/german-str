#![cfg(target_pointer_width = "64")]

use std::alloc::handle_alloc_error;
use std::borrow::{Borrow, Cow};
use std::ops::Deref;
use std::ptr::{null, NonNull};
use std::str::FromStr;

/// The maximum number of chars a GermanStr can contain before requiring a heap allocation.
pub const MAX_INLINE_BYTES: usize = 12;

/// The absolute maximum number of chars a GermanStr can hold.
/// Since the len is an u32, it is 2^32.
pub const MAX_LEN: usize = 2_usize.pow(32);

#[repr(C)]
pub struct GermanStr {
    /// Number of chars of the string.
    len: u32,

    /// The first 4 bytes of the string.
    /// If the string has less than 4 bytes, the extra bytes are set to 0.
    /// Since an UTF-8 char can consist of 1-4 bytes, it cannot be interpreted
    /// as chars.
    /// This prefix can still be used to speed up comparisons because UTF-8 strings
    /// are sorted byte-wise.
    prefix: u32,

    /// If the string is longer than 12 bytes, is an owning, unique pointer to the
    /// chars on the heap.
    /// Otherwise, is an `[u8; 8]`, with extra bytes set to 0.
    /// The prefix is also included on the heap.
    ptr: *const u8,
}

// Safety: According to the rustonomicon, "something can safely be Send unless it shares mutable
// state with something else without enforcing exclusive access to it."
// The `ptr` is never shared between `GermanStr`, so there's no shared mutable state.
unsafe impl Send for GermanStr {}

// Safety: Again, according to the rustonomicon, there's no issue here since GermanStr are
// immutable.
unsafe impl Sync for GermanStr {}

impl Drop for GermanStr {
    #[inline]
    fn drop(&mut self) {
        if self.len as usize > MAX_INLINE_BYTES {
            let ptr = self.ptr.cast_mut();
            unsafe {
                // Safety: this call can only fail if self.len is too long.
                // We can only create a long `GermanStr` using GermanStr::new: if `self.len`
                // was too long, we'd get an error when we try to create the GermanStr.
                let layout = std::alloc::Layout::array::<u8>(self.len as usize).unwrap_unchecked();
                std::alloc::dealloc(ptr, layout);
            }
        }
        // In the case where len <= MAX_INLINE_BYTES, no heap data is owned and
        // no deallocation is needed.
    }
}

impl Clone for GermanStr {
    #[inline]
    fn clone(&self) -> Self {
        if self.len as usize <= MAX_INLINE_BYTES {
            GermanStr {
                len: self.len,
                prefix: self.prefix,
                ptr: self.ptr,
            }
        } else {
            GermanStr::new(self.as_str()).unwrap()
        }
    }
}

impl GermanStr {
    #[inline]
    pub fn new<T>(src: T) -> Option<Self>
    where
        T: AsRef<str>,
    {
        let src = src.as_ref();
        if src.len() > MAX_LEN {
            return None;
        }
        if src.len() <= MAX_INLINE_BYTES {
            return Some(GermanStr::new_inline(src));
        }

        let prefix = str_prefix::<&str>(&src);
        let layout = std::alloc::Layout::array::<u8>(src.len()).ok()?;
        let ptr = unsafe { std::alloc::alloc(layout) };
        if ptr.is_null() {
            handle_alloc_error(layout);
        }
        let slice = unsafe { std::slice::from_raw_parts_mut(ptr, src.len()) };
        slice.clone_from_slice(src.as_bytes());
        Some(GermanStr {
            len: src.len() as u32,
            prefix,
            ptr,
        })
    }

    #[inline]
    pub const fn new_inline(src: &str) -> GermanStr {
        assert!(src.len() <= MAX_INLINE_BYTES);
        let res = GermanStr {
            len: src.len() as u32,
            prefix: 0,
            ptr: null(),
        };
        unsafe {
            let mut as_buf: [u8; 16] = std::mem::transmute(res);
            let mut i = 0;
            while i < src.len() {
                as_buf[4 + i] = src.as_bytes()[i];
                i += 1;
            }
            std::mem::transmute(as_buf)
        }
    }

    #[inline]
    pub fn prefix(&self) -> &[u8] {
        let prefix_len = self.len().min(4) as usize;
        let prefix_addr: *const u32 = &self.prefix;
        unsafe {
            let ptr = std::mem::transmute(prefix_addr);
            std::slice::from_raw_parts(ptr, prefix_len)
        }
    }

    #[inline]
    pub fn suffix(&self) -> &[u8] {
        let suffix_len = self.len().saturating_sub(4) as usize;
        if self.len() <= MAX_INLINE_BYTES {
            unsafe {
                let ptr = if self.ptr.is_null() {
                    NonNull::dangling().as_ptr()
                } else {
                    let ptr_addr = &self.ptr as *const *const u8;
                    std::mem::transmute(ptr_addr)
                };
                std::slice::from_raw_parts(ptr, suffix_len)
            }
        } else {
            unsafe {
                std::slice::from_raw_parts(self.ptr.add(4), suffix_len)
            }
        }
    }

    #[inline(always)]
    pub fn as_str(&self) -> &str {
        Deref::deref(self)
    }

    #[inline(always)]
    pub fn to_string(&self) -> String {
        self.as_str().to_owned()
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len as usize
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool  {
        self.len == 0
    }

    #[inline(always)]
    pub const fn is_heap_allocated(&self) -> bool {
        self.len as usize > MAX_INLINE_BYTES
    }
}

impl Deref for GermanStr {
    type Target = str;

    #[inline(always)]
    fn deref(&self) -> &str {
        let len = self.len as usize;
        if len <= MAX_INLINE_BYTES {
            let prefix_addr: *const u32 = &self.prefix;
            unsafe {
                let ptr = std::mem::transmute(prefix_addr);
                let slice = std::slice::from_raw_parts(ptr, len);
                std::str::from_utf8_unchecked(slice)
            }
        } else {
            unsafe {
                let slice = std::slice::from_raw_parts(self.ptr, len);
                std::str::from_utf8_unchecked(slice)
            }
        }
    }
}

impl PartialEq<GermanStr> for GermanStr {
    #[inline(always)]
    fn eq(&self, other: &GermanStr) -> bool {
        self.prefix == other.prefix && self.suffix() == other.suffix()
    }
}

impl Ord for GermanStr {
    #[cfg(target_endian = "little")]
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.prefix.swap_bytes().cmp(&other.prefix.swap_bytes())
            .then_with(|| self.suffix().cmp(other.suffix()))
    }

    #[cfg(target_endian = "big")]
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.prefix.cmp(&other.prefix)
            .then_with(|| self.suffix().cmp(other.suffix()))
    }
}


impl Eq for GermanStr {}

impl PartialEq<str> for GermanStr {
    #[inline(always)]
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<GermanStr> for str {
    #[inline(always)]
    fn eq(&self, other: &GermanStr) -> bool {
        other == self
    }
}

impl<'a> PartialEq<&'a str> for GermanStr {
    #[inline(always)]
    fn eq(&self, other: &&'a str) -> bool {
        self == *other
    }
}

impl<'a> PartialEq<GermanStr> for &'a str {
    #[inline(always)]
    fn eq(&self, other: &GermanStr) -> bool {
        *self == other
    }
}

impl PartialEq<String> for GermanStr {
    #[inline(always)]
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<GermanStr> for String {
    #[inline(always)]
    fn eq(&self, other: &GermanStr) -> bool {
        other.prefix == str_prefix::<&String>(self)
            && other.suffix() == str_suffix::<&String>(self)
    }
}

impl<'a> PartialEq<&'a String> for GermanStr {
    #[inline(always)]
    fn eq(&self, other: &&'a String) -> bool {
        self == *other
    }
}

impl<'a> PartialEq<GermanStr> for &'a String {
    #[inline(always)]
    fn eq(&self, other: &GermanStr) -> bool {
        *self == other
    }
}

impl PartialOrd for GermanStr {
    #[inline(always)]
    fn partial_cmp(&self, other: &GermanStr) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Default for GermanStr {
    #[inline(always)]
    fn default() -> GermanStr {
        GermanStr {
            len: 0,
            prefix: 0,
            ptr: null(),
        }
    }
}

impl std::hash::Hash for GermanStr {
    #[inline(always)]
    fn hash<H: std::hash::Hasher>(&self, hasher: &mut H) {
        self.as_str().hash(hasher);
    }
}

impl std::fmt::Display for GermanStr {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(self.as_str(), f)
    }
}

/*
impl iter::FromIterator<char> for GermanStr {
    fn from_iter<I: iter::IntoIterator<Item = char>>(iter: I) -> GermanStr {
        let iter = iter.into_iter();
        Self::from_char_iter(iter)
    }
}
*/

impl TryFrom<&str> for GermanStr {
    type Error = ();

    #[inline]
    fn try_from(s: &str) -> Result<GermanStr, ()> {
        GermanStr::new(s).ok_or(())
    }
}

impl TryFrom<&mut str> for GermanStr {
    type Error = ();

    #[inline]
    fn try_from(s: &mut str) -> Result<GermanStr, ()> {
        GermanStr::new(s).ok_or(())
    }
}

impl TryFrom<&String> for GermanStr {
    type Error = ();

    #[inline]
    fn try_from(s: &String) -> Result<GermanStr, ()> {
        GermanStr::new(s).ok_or(())
    }
}

impl TryFrom<String> for GermanStr {
    type Error = ();

    #[inline(always)]
    fn try_from(text: String) -> Result<Self, ()> {
        Self::new(text).ok_or(())
    }
}

impl<'a> TryFrom<Cow<'a, str>> for GermanStr {
    type Error = ();

    #[inline]
    fn try_from(s: Cow<'a, str>) -> Result<GermanStr, ()> {
        GermanStr::new(s).ok_or(())
    }
}

/*
impl From<GermanStr> for Arc<str> {
    #[inline(always)]
    fn from(text: GermanStr) -> Self {
        match text.0 {
            Repr::Heap(data) => data,
            _ => text.as_str().into(),
        }
    }
}
*/

impl From<GermanStr> for String {
    #[inline(always)]
    fn from(text: GermanStr) -> Self {
        text.as_str().into()
    }
}

impl Borrow<str> for GermanStr {
    #[inline(always)]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for GermanStr {
    type Err = ();

    #[inline]
    fn from_str(s: &str) -> Result<GermanStr, Self::Err> {
        GermanStr::new(s).ok_or(())
    }
}

impl std::fmt::Debug for GermanStr {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_str(), f)
    }
}

#[inline(always)]
pub fn str_suffix<T>(src: &impl AsRef<str>) -> &[u8] {
    src.as_ref().as_bytes().get(4..).unwrap_or_default()
}

#[inline(always)]
pub fn str_prefix<T>(src: impl AsRef<str>) -> u32 {
    let src = src.as_ref().as_bytes();
    let mut bytes = [0; 4];
    for i in 0..src.len().min(4) {
        bytes[i] = src[i];
    }
    unsafe {
        std::mem::transmute(bytes)
    }
}
