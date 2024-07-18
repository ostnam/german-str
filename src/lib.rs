use std::borrow::{Borrow, Cow};
use std::ops::Deref;
use std::ptr::null;
use std::str::FromStr;

pub const MAX_INLINE_CHARS: usize = 12;
pub const MAX_LEN: usize = 2_usize.pow(32);

#[repr(C)]
pub struct GermanStr {
    len: u32,
    prefix: u32,
    ptr: *const u8,
}

impl Drop for GermanStr {
    fn drop(&mut self) {
        if self.len as usize > MAX_INLINE_CHARS {
            unsafe {
                let ptr = std::mem::transmute(self.ptr);
                let layout  = std::alloc::Layout::array::<u8>(self.len as usize).unwrap_unchecked();
                std::alloc::dealloc(ptr, layout);
            }
        }
    }
}

impl Clone for GermanStr {
    fn clone(&self) -> Self {
        if self.len as usize <= MAX_INLINE_CHARS {
            let mut res = GermanStr::default();
            unsafe {
                std::ptr::copy_nonoverlapping(self, &mut res, 1);
            }
            res
        } else {
            GermanStr::new(self.as_str()).unwrap()
        }
    }
}

impl GermanStr {
    pub fn new<T>(src: T) -> Option<Self>
    where
        T: AsRef<str>,
    {
        let src = src.as_ref();
        if src.len() > MAX_LEN {
            return None;
        }
        if src.len() <= MAX_INLINE_CHARS {
            return Some(GermanStr::new_inline(src));
        }
        let prefix = unsafe {
            let mut buf = [0; 4];
            for i in 0..src.len().min(4) {
                buf[i] = src.as_bytes()[i];
            }
            std::mem::transmute(buf)
        };
        let ptr = unsafe {
            let layout  = std::alloc::Layout::array::<u8>(src.len()).ok()?;
            let buf: *mut u8 = std::alloc::alloc(layout);
            let as_slice = std::slice::from_raw_parts_mut(buf, src.len());
            as_slice[..].clone_from_slice(src.as_bytes());
            buf
        };
        Some(GermanStr {
            len: src.len() as u32,
            prefix,
            ptr,
        })
    }

    #[inline]
    pub const fn new_inline(src: &str) -> GermanStr {
        assert!(src.len() <= MAX_INLINE_CHARS);
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
        if self.len() <= MAX_INLINE_CHARS {
            unsafe {
                std::slice::from_raw_parts(self.ptr, suffix_len)
            }
        } else {
            unsafe {
                std::slice::from_raw_parts(self.ptr.add(4), suffix_len)
            }
        }
    }

    #[inline]
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
        self.len as usize > MAX_INLINE_CHARS
    }

    fn from_char_iter() {
        todo!()
    }
}

impl Deref for GermanStr {
    type Target = str;

    #[inline(always)]
    fn deref(&self) -> &str {
        let len = self.len as usize;
        if len <= MAX_INLINE_CHARS {
            unsafe {
                let prefix_addr: *const u32 = &self.prefix;
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
    fn eq(&self, other: &GermanStr) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for GermanStr {}


impl PartialEq<str> for GermanStr {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<GermanStr> for str {
    fn eq(&self, other: &GermanStr) -> bool {
        other == self
    }
}

impl<'a> PartialEq<&'a str> for GermanStr {
    fn eq(&self, other: &&'a str) -> bool {
        self == *other
    }
}

impl<'a> PartialEq<GermanStr> for &'a str {
    fn eq(&self, other: &GermanStr) -> bool {
        *self == other
    }
}

impl PartialEq<String> for GermanStr {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<GermanStr> for String {
    fn eq(&self, other: &GermanStr) -> bool {
        other == self
    }
}

impl<'a> PartialEq<&'a String> for GermanStr {
    fn eq(&self, other: &&'a String) -> bool {
        self == *other
    }
}

impl<'a> PartialEq<GermanStr> for &'a String {
    fn eq(&self, other: &GermanStr) -> bool {
        *self == other
    }
}

impl Ord for GermanStr {
    #[cfg(target_endian = "little")]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.prefix.swap_bytes().cmp(&other.prefix.swap_bytes())
            .then_with(|| self.suffix().cmp(other.suffix()))
    }

    #[cfg(target_endian = "big")]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.prefix.cmp(&other.prefix)
            .then_with(|| self.suffix().cmp(other.suffix()))
    }
}


impl PartialOrd for GermanStr {
    fn partial_cmp(&self, other: &GermanStr) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::hash::Hash for GermanStr {
    fn hash<H: std::hash::Hasher>(&self, hasher: &mut H) {
        self.as_str().hash(hasher);
    }
}

impl std::fmt::Display for GermanStr {
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

impl std::fmt::Debug for GermanStr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_str(), f)
    }
}
