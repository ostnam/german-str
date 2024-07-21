# GermanStr
German strings are a string type with the follow properties:

* They are immutable.
* `size_of::<GermanStr>() == 16`
* They can't be longer than 2^32 bytes.
* Strings of 12 or less bytes are entirely located on the stack.
* Comparisons depending only on the first 4 bytes are very fast.

They are described [here](https://cedardb.com/blog/german_strings/). TL;DR: it's a 16 bytes struct where:
  * The first 4 bytes of the struct store the length of the string.
  * The first 4 bytes of the string are stored right after.
  * If the rest of the string can fit in the remaining 8 bytes, it is directly stored there.
  * Otherwise the last 8 bytes are a pointer to the string buffer on the heap (which includes the 4 bytes prefix).

The main difference between this article and this implementation is that we don't tag the pointer with a "storage class".

The implementation was heavily inspired by SmolStr.

# Requirements.
* `[cfg(target_pointer_width = "64")]`
* The crate is compatible with `[no_std]`.
