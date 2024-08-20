#![no_std]
#![cfg(target_pointer_width = "64")]

extern crate alloc;

use alloc::borrow::{Cow, ToOwned as _};
use alloc::boxed::Box;
use alloc::slice;
use alloc::string::String;
use alloc::sync::Arc;
use core::{cmp, fmt, ptr};
use core::alloc::Layout;
use core::borrow::Borrow;
use core::ops::Deref;
use core::ptr::NonNull;
use core::str::FromStr;

/// The maximum number of chars a GermanStr can contain before requiring
/// a heap allocation.
pub const MAX_INLINE_BYTES: usize = 12;

/// The absolute maximum number of chars a GermanStr can hold.
/// Since the len is an u32, it is 2^32.
pub const MAX_LEN: usize = 2_usize.pow(32);

/// Stored in stolen bits of the heap pointer, to indicate that it is an
/// owned pointer and its heap allocation should be freed on drop.
const OWNED_PTR: usize = 0;

/// Stored in the stolen bits of the heap pointer, to indicate that it is a
/// shared buffer and that the user is responsible for freeing it.
const SHARED_PTR: usize = usize::MAX;

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

    /// If the string is longer than 12 bytes, is a pointer to
    /// the chars on the heap.
    /// By default, this pointer is unique and has ownership of the allocation,
    /// but the heap buffer can be shared if `leaky_shared_clone` is called,
    /// in which case you are then responsible for freeing it correctly.
    /// The prefix is also included in the buffer.
    ///
    /// If the string fits in 12 bytes, is an `[u8; 8]`, with extra bytes
    /// set to 0 (the first 4 bytes being stored in `self.prefix`).
    last8: Last8,
}

#[derive(Copy, Clone)]
/// Holds the last 8 bytes of a `GermanStr`.
union Last8 {
    /// Non-null pointer to u8 with 1 bit of virtual address space stolen.
    ptr: ointers::NotNull<u8, 0, false, 1>,
    // Safety:
    // "If compiling for a 64bit arch, V must be at most 25": we have
    // #![cfg(target_pointer_width = "64")] and V == 1.

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
        let Some(ptr) = NonNull::new(ptr) else {
            alloc::alloc::handle_alloc_error(layout);
        };
        unsafe {
            // Safety:
            //   1. We assume src is a valid object.
            //   2. ptr is valid: it was checked for null and allocated
            //      for src.len() bytes.
            //   3. *_ u8 is always aligned.
            //   4. The 2 regions can't overlap since they belong to different objects.
            ptr::copy_nonoverlapping(
                src.as_bytes().as_ptr(),
                ptr.as_ptr(),
                src.len(),
            );
        }
        let ointer = unsafe {
            // Safety: see Last8.ptr declaration.
            ointers::NotNull::new_stealing(ptr, OWNED_PTR)
        };
        Ok(GermanStr {
            len: src.len() as u32,
            prefix: str_prefix::<&str>(&src),
            last8: Last8 { ptr: ointer },
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

    #[inline(always)]
    /// Returns the pointer to the heap-allocated buffer, if the `GermanStr`
    /// isn't inlined.
    /// In the actual GermanStr, 1 bit of the pointer is stolen to store
    /// whether the heap allocation is shared or owned. Here, that bit is
    /// reset to its default value before the pointer is returned.
    /// `GermanStr::is_shared` can be used if you want to access that bit's
    /// value.
    pub fn heap_ptr(&self) -> Option<NonNull<u8>> {
        self.heap_ointer()
            .map(|ointer| ointer.as_non_null())
    }

    #[inline(always)]
    /// Safe accessor for `self.last8.ptr`.
    fn heap_ointer(&self) -> Option<ointers::NotNull<u8, 0, false, 1>> {
        if self.len as usize > MAX_INLINE_BYTES {
            Some(unsafe {
                    // Safety: self.len > MAX_INLINE_BYTES => self isn't inlined.
                    self.last8.ptr
            })
        } else {
            None
        }
    }


    #[inline(always)]
    /// Returns whether `self` is heap-allocated, and the buffer possibly
    /// shared with other instances, as after calling `leaky_shared_clone`.
    pub fn has_shared_buffer(&self) -> bool {
        self.heap_ointer().is_some_and(|ptr| ptr.stolen() != OWNED_PTR)
    }

    #[inline]
    /// Clones `self`, reusing the same heap-allocated buffer (unless `self`
    /// is inlined).
    ///
    /// After calling this method, the heap buffer will not be freed when
    /// `self` is `Drop`ped: you are responsible for manually managing
    /// memory by calling `GermanStr::free` exactly once per heap buffer.
    ///
    /// Even after calling this method once, it should be called instead of
    /// clone to make new copies that reuse the same buffer: calling `clone()`
    /// will always create a new copy of the buffer.
    ///
    /// This can save memory and increase performance in the case where you
    /// have many equal `GermanStr` longer than `MAX_INLINE_BYTES`.
    pub fn leaky_shared_clone(&mut self) -> Self {
        if self.is_heap_allocated() {
            unsafe {
                self.last8.ptr = self.last8.ptr.steal(SHARED_PTR);
            }
        }
        GermanStr {
            len: self.len,
            prefix: self.prefix,
            last8: self.last8,
        }
    }

    /// Should be called to free the heap buffer of a shared `GermanStr`.
    ///
    /// # Safety
    /// * `self` should be heap-allocated and not inlined (you can check with
    /// `GermanStr::is_heap_allocated`).
    /// * You should only free each buffer once.
    ///
    /// However, `free()`ing a heap allocated but non-shared `GermanStr` is
    /// safe and equivalent to dropping it.
    ///
    /// To avoid double frees, you can simply store a set of freed pointers.
    /// ```no_run
    /// use std::collections::BTreeSet;
    /// # use german_str::GermanStr;
    ///
    /// let mut freed = BTreeSet::new();
    /// # let german_str: Vec<GermanStr> = Vec::new();
    /// for s in german_str {
    ///     let Some(ptr) = s.heap_ptr() else {
    ///         continue; // skip inlined GermanStr
    ///     };
    ///     if freed.insert(ptr) {
    ///         unsafe {
    ///             // Safety:
    ///             // 1. s is heap-allocated or s.heap_ptr() would be None.
    ///             // 2. If ptr had already been freed, insert()'ing it would've returned false.
    ///             s.free();
    ///         }
    ///     } else {
    ///         std::mem::forget(s);
    ///     }
    /// }
    /// ```
    pub unsafe fn free(mut self) {
        unsafe {
            // Safety:
            // the caller is responsible for checking that `self` isn't inlined.
            self.last8.ptr = self.last8.ptr.steal(OWNED_PTR);
        }
        core::mem::drop(self)
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
                // Safety:
                // self.len  > MAX_INLINE_BYTES => self.last8 is heap ptr.
                let ptr = self.last8.ptr.as_non_null().as_ptr();

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
    /// Returns whether a heap allocation is used to store the string.
    pub const fn is_heap_allocated(&self) -> bool {
        self.len as usize > MAX_INLINE_BYTES
    }

    #[inline(always)]
    /// Returns whether the string is stored entirely within `self`, without a heap allocation.
    pub const fn is_inlined(&self) -> bool {
        !self.is_heap_allocated()
    }
}

impl Clone for GermanStr {
    #[inline]
    fn clone(&self) -> Self {
        if let Some(self_ptr) = self.heap_ptr() {
            let (ptr, layout) = unsafe {
                // Safety: If len was too high for this layout, we couldn't
                // have made self in the first place.
                let layout = Layout::array::<u8>(self.len()).unwrap_unchecked();

                // Safety: layout is not zero-sized, otherwise we would store the string inplace.
                let ptr = alloc::alloc::alloc(layout);
                (ptr, layout)
            };
            let Some(ptr) = NonNull::new(ptr) else {
                alloc::alloc::handle_alloc_error(layout);
            };
            unsafe {
                // Safety:
                //   1. Both pointers are valid.
                //   2. *_ u8 is always aligned.
                //   3. The 2 regions can't overlap since they belong to different objects.
                ptr::copy_nonoverlapping(
                    self_ptr.as_ptr(),
                    ptr.as_ptr(),
                    self.len(),
                );
            }
            let ointer = unsafe {
                // Safety: see Last8.ptr declaration.
                ointers::NotNull::new_stealing(ptr, OWNED_PTR)
            };
            GermanStr {
                prefix: self.prefix,
                len: self.len,
                last8: Last8 { ptr: ointer },
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
        let ptr = match self.heap_ptr() {
            Some(ptr) if !self.has_shared_buffer() => ptr,
            Some(_) | None => return,
            // If the heap buffer is shared, or the string is inlined,
            // dropping should be a no-op.
        };
        unsafe {
            // Safety: this call can only fail if self.len is too long.
            // We can only create a long `GermanStr` using GermanStr::new: if `self.len`
            // was too long, we'd get an error when we try to create the GermanStr.
            let layout = Layout::array::<u8>(self.len as usize).unwrap_unchecked();
            alloc::alloc::dealloc(ptr.as_ptr(), layout);
        }
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
        let ptr = self.heap_ptr()
            .unwrap_or_else(|| unsafe {
                // Safety:
                // self.prefix can't be null since it comes from &self.
                NonNull::new_unchecked(self.prefix.as_ptr().cast_mut())
            });
        unsafe {
            // Safety:
            // * Since we're making a &[u8], it is guaranteed to be aligned.
            // * The pointer used is NonNull and part of a single object (self
            // or the heap buffer).
            // * The len of the slice is correct.
            // * The len is shorter than isize::MAX (2^63 - 1, MAX_LEN == 2^32).
            // * ptr + len < isize::MAX, or the heap buffer/struct would overflow usize::MAX.
            let slice = slice::from_raw_parts(ptr.as_ptr(), self.len as usize);

            // Safety:
            // A `GermanStr` is guaranteed to be a valid UTF8 string,
            // since it can only be constructed from an impl AsRef<str>,
            // a String, or a Writer that accepts &str.
            core::str::from_utf8_unchecked(slice)
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
        let prefixes_equal = self.prefix == other.prefix;
        if !prefixes_equal {
            return false;
        } else if self.len <= 4 && other.len <= 4 {
            return true;
        }

        if self.is_inlined() && other.is_inlined() {
            return unsafe {
                // Safety: obviously both strings are stored inline.
                self.last8.buf == other.last8.buf
            };
        }

        return self.suffix_bytes_slice() == other.suffix_bytes_slice();
    }
}

impl Eq for GermanStr {}

impl Ord for GermanStr {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.prefix
            .cmp(&other.prefix)
            .then_with(||
                if self.len <= 4 && other.len <= 4 {
                    cmp::Ordering::Equal
                } else if self.is_inlined() && other.is_inlined() {
                    unsafe {
                        // Safety: obviously both strings are stored inline.
                        self.last8.buf.cmp(&other.last8.buf)
                    }
                } else {
                    self.suffix_bytes_slice().cmp(other.suffix_bytes_slice())
                }
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

/// Almost identical to [`ToString`], but converts to `GermanStr` instead.
pub trait ToGermanStr {
    fn to_german_str(&self) -> GermanStr;
}

#[doc(hidden)]
pub struct Writer {
    len: usize,
    inline: [u8; MAX_INLINE_BYTES],
    heap: String,
}

impl Writer {
    #[must_use]
    pub const fn new() -> Self {
        Writer {
            len: 0,
            inline: [0; MAX_INLINE_BYTES],
            heap: String::new(),
        }
    }

    fn push_str(&mut self, s: &str) -> Result<(), InitError> {
        let old_len = self.len;
        self.len += s.len();
        if self.len > MAX_LEN {
            return Err(InitError::TooLong);
        }
        if self.len <= MAX_INLINE_BYTES {
            // we are still inline after the write
            self.inline[old_len..self.len].copy_from_slice(s.as_bytes());
        } else if old_len <= MAX_INLINE_BYTES {
            // we need to switch from inline to heap
            self.heap.reserve(self.len);
            unsafe {
                // SAFETY: inline data is guaranteed to be valid utf8 for previously
                // written bytes since this is the only &mut method and we write an
                // entire &str at each call, which is valid UTF8 bytes.
                self.heap
                    .as_mut_vec()
                    .extend_from_slice(&self.inline[..old_len]);
            }
            self.heap.push_str(s);
        } else {
            self.heap.push_str(s);
        }
        Ok(())
    }
}

impl fmt::Write for Writer {
    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s)
            .map_err(|_| fmt::Error::default())
    }
}

/// Formats arguments to a [`GermanStr`], potentially without allocating.
///
/// See [`alloc::format!`] or [`format_args!`] for syntax documentation.
#[macro_export]
macro_rules! format_german_str {
    ($($tt:tt)*) => {{
        use ::core::fmt::Write;
        let mut w = $crate::Writer::new();
        w.write_fmt(format_args!($($tt)*))
            .expect("tried to format_german_str a GermanStr bigger than the maximum GermanStr size");
        $crate::GermanStr::from(w)
    }};
}

impl From<Writer> for GermanStr {
    fn from(value: Writer) -> Self {
        if value.len <= MAX_INLINE_BYTES {
            let mut prefix = [0; 4];
            prefix.clone_from_slice(&value.inline[0..4]);
            let mut last8 = [0; 8];
            last8.clone_from_slice(&value.inline[4..MAX_INLINE_BYTES]);
            GermanStr {
                len: value.len as u32,
                prefix,
                last8: Last8 { buf: last8 },
            }
        } else {
            let heap_ref = value.heap.leak(); // avoid copying the str
            let non_null = unsafe {
                NonNull::new_unchecked(heap_ref.as_mut_ptr())
            };
            let ointer = unsafe {
                // Safety: see Last8.ptr declaration.
                ointers::NotNull::new_stealing(non_null, OWNED_PTR)
            };
            GermanStr {
                len: value.len as u32,
                prefix: str_prefix::<&str>(heap_ref),
                last8: Last8 { ptr: ointer },
            }
        }
    }
}

impl<T> ToGermanStr for T
where
    T: fmt::Display + ?Sized,
{
    fn to_german_str(&self) -> GermanStr {
        format_german_str!("{}", self)
    }
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for GermanStr {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> Result<Self, arbitrary::Error> {
        let s = <&str>::arbitrary(u)?;
        Ok(GermanStr::new(s).expect("BUG in arbitrary implementation of GermanStr. Please report it at github.com/ostnam/german-str/issues"))
    }

    fn size_hint(_: usize) -> (usize, Option<usize>) {
        (0, Some(MAX_LEN))
    }
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
