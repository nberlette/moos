//! A UTF‑8 string type with inline storage.
//!
//! `SmallString<N>` stores up to `N` bytes of UTF‑8 inline, falling
//! back to dynamic allocation for longer strings. It dereferences to
//! `str` and provides familiar string operations. When compiled with
//! the optional `serde` feature, it serializes as a normal string.
//!
//! ## Examples
//!
//! Creating a `SmallString` and appending characters:
//!
//! ```
//! use moos::SmallString;
//!
//! let mut s: SmallString<8> = SmallString::new();
//! s.push_str("hi");
//! s.push('!');
//! assert_eq!(s.as_str(), "hi!");
//! assert!(s.is_inline());
//! ```
//!
//! Pushing a long string will cause the inline storage to spill to the
//! heap:
//!
//! ```
//! use moos::SmallString;
//!
//! let mut s: SmallString<4> = SmallString::new();
//! s.push_str("abcdef");
//! // 6 bytes exceed the inline capacity of 4
//! assert!(!s.is_inline());
//! assert_eq!(s.as_str(), "abcdef");
//! ```

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::ops::Deref;
use core::ops::DerefMut;

use crate::compact_vec::CompactVec;

/// A UTF‑8 string with inline storage for up to `N` bytes.
///
/// `SmallString<N>` behaves similarly to `String`, but uses a
/// [`CompactVec<u8, N>`] internally to minimize heap allocations for
/// short strings. It implements `Deref<Target = str>` so most string
/// operations are available via the deref coercion.
pub struct SmallString<const N: usize> {
  inner: CompactVec<u8, N>,
}

impl<const N: usize> SmallString<N> {
  /// Creates a new empty `SmallString` with inline capacity `N`.
  pub fn new() -> Self {
    Self {
      inner: CompactVec::new(),
    }
  }

  /// Returns the length of the string in bytes.
  pub const fn len(&self) -> usize {
    self.inner.len()
  }

  /// Returns `true` if the string is empty.
  pub fn is_empty(&self) -> bool {
    self.inner.is_empty()
  }

  /// Returns `true` if the underlying storage is currently inline.
  pub fn is_inline(&self) -> bool {
    self.inner.is_inline()
  }

  /// Returns the total capacity of the underlying storage in bytes.
  pub fn capacity(&self) -> usize {
    self.inner.capacity()
  }

  /// Returns the string as a `&str`. This panics if the internal
  /// storage contains invalid UTF‑8; all methods that push bytes
  /// ensure that only valid UTF‑8 is stored so this is only relevant
  /// when unsafely constructing a `SmallString`.
  pub fn as_str(&self) -> &str {
    // SAFETY: We only ever push valid UTF‑8 bytes through the public
    // API, so interpreting the inner slice as a str is safe.
    unsafe { core::str::from_utf8_unchecked(self.inner.as_slice()) }
  }

  /// Returns the string as a mutable `&mut str`. This is safe
  /// because the returned slice cannot grow to include uninitialized
  /// bytes and any mutation must preserve valid UTF‑8.
  pub fn as_mut_str(&mut self) -> &mut str {
    unsafe { core::str::from_utf8_unchecked_mut(self.inner.as_mut_slice()) }
  }

  /// Appends a single character to the end of the string. This may
  /// spill to the heap if the inline capacity is exceeded. Panics if
  /// the resulting string would exceed `usize::MAX` bytes.
  pub fn push(&mut self, c: char) {
    let mut buf = [0u8; 4];
    let encoded = c.encode_utf8(&mut buf);
    self.push_str(encoded);
  }

  /// Appends a string slice to the end of the string. Spills to the
  /// heap if necessary.
  pub fn push_str(&mut self, s: &str) {
    self.inner.extend(s.as_bytes().iter().copied());
  }

  /// Consumes the `SmallString` and returns a standard `String` with
  /// identical contents.
  pub fn into_string(self) -> String {
    // Convert the inner CompactVec<u8, N> into a Vec<u8> and then
    // unsafely interpret it as a String. Since we ensure that
    // all pushed bytes form valid UTF‑8, this operation is safe.
    let bytes: Vec<u8> = self.inner.into_vec();
    // SAFETY: The contents of `bytes` are guaranteed to be valid
    // UTF‑8 because we only ever append valid UTF‑8 through the
    // public API.
    unsafe { String::from_utf8_unchecked(bytes) }
  }

  /// Creates a `SmallString` from a string slice. The string will
  /// store as many bytes inline as possible and spill to the heap
  /// automatically if the capacity is exceeded.
  pub fn from_str(s: &str) -> Self {
    let mut inner = CompactVec::with_capacity(s.len());
    inner.extend(s.as_bytes().iter().copied());
    Self { inner }
  }
}

impl<const N: usize> Default for SmallString<N> {
  fn default() -> Self {
    Self::new()
  }
}

impl<const N: usize> fmt::Debug for SmallString<N> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Debug::fmt(self.as_str(), f)
  }
}

impl<const N: usize> fmt::Display for SmallString<N> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(self.as_str())
  }
}

impl<const N: usize> Clone for SmallString<N> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<const N: usize> Deref for SmallString<N> {
  type Target = str;
  fn deref(&self) -> &Self::Target {
    self.as_str()
  }
}

impl<const N: usize> DerefMut for SmallString<N> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.as_mut_str()
  }
}

impl<const N: usize> From<String> for SmallString<N> {
  fn from(s: String) -> Self {
    Self::from_str(&s)
  }
}

impl<const N: usize> From<&str> for SmallString<N> {
  fn from(s: &str) -> Self {
    Self::from_str(s)
  }
}

impl<const N: usize> From<SmallString<N>> for String {
  fn from(s: SmallString<N>) -> Self {
    s.into_string()
  }
}

impl<const N: usize> PartialEq<str> for SmallString<N> {
  fn eq(&self, other: &str) -> bool {
    self.as_str() == other
  }
}

impl<const N: usize> PartialEq for SmallString<N> {
  fn eq(&self, other: &Self) -> bool {
    self.as_str() == other.as_str()
  }
}

impl<const N: usize> Eq for SmallString<N> {}

impl<const N: usize> PartialOrd for SmallString<N> {
  fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
    Some(self.as_str().cmp(other.as_str()))
  }
}

impl<const N: usize> Ord for SmallString<N> {
  fn cmp(&self, other: &Self) -> core::cmp::Ordering {
    self.as_str().cmp(other.as_str())
  }
}

impl<const N: usize> core::hash::Hash for SmallString<N> {
  fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
    self.as_str().hash(state)
  }
}

#[cfg(feature = "serde")]
mod serde_impl {
  use super::*;

  impl<const N: usize> serde::Serialize for SmallString<N> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
      S: serde::Serializer,
    {
      serializer.serialize_str(self.as_str())
    }
  }

  impl<'de, const N: usize> serde::Deserialize<'de> for SmallString<N> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
      D: serde::Deserializer<'de>,
    {
      let s = <&str>::deserialize(deserializer)?;
      Ok(SmallString::from_str(s))
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn basic_usage() {
    let mut s: SmallString<4> = SmallString::new();
    assert!(s.is_empty());
    assert!(s.is_inline());
    s.push_str("hi");
    s.push('!');
    assert_eq!(s.as_str(), "hi!");
    assert!(s.is_inline());
  }

  #[test]
  fn spill_and_convert() {
    let mut s: SmallString<4> = SmallString::new();
    s.push_str("abcd");
    assert!(s.is_inline());
    // This push exceeds the inline capacity and spills
    s.push('e');
    assert!(!s.is_inline());
    assert_eq!(s.as_str(), "abcde");
    // Convert into a String and compare
    let owned: String = s.clone().into();
    assert_eq!(owned, "abcde");
  }

  #[test]
  fn from_and_into_str() {
    let original = "hello world";
    let s: SmallString<5> = SmallString::from(original);
    assert_eq!(s.as_str(), original);
    let converted: String = s.clone().into();
    assert_eq!(converted, original);
  }

  #[test]
  fn empty_capacity_and_mutation() {
    let mut s: SmallString<3> = SmallString::new();
    assert!(s.is_empty());
    assert_eq!(s.len(), 0);
    assert!(s.is_inline());
    assert_eq!(s.capacity(), 3);

    s.push_str("hi");
    s.push('!');
    assert_eq!(s.as_str(), "hi!");
    let s_mut: &mut str = s.as_mut_str();
    s_mut.make_ascii_uppercase();
    assert_eq!(s.as_str(), "HI!");
  }

  #[test]
  fn from_str_spills_and_converts() {
    let s: SmallString<4> = SmallString::from("abcdef");
    assert_eq!(s.as_str(), "abcdef");
    assert!(!s.is_inline());

    let owned = s.clone().into_string();
    assert_eq!(owned, "abcdef");
    let converted: String = s.into();
    assert_eq!(converted, "abcdef");
  }

  #[test]
  fn ordering_hash_and_eq() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let a: SmallString<4> = SmallString::from("apple");
    let b: SmallString<4> = SmallString::from("banana");
    assert!(a < b);
    assert_eq!(a.as_str(), "apple");
    let a_clone = a.clone();
    assert_eq!(a, a_clone);

    let mut h1 = DefaultHasher::new();
    a.hash(&mut h1);
    let mut h2 = DefaultHasher::new();
    a_clone.hash(&mut h2);
    assert_eq!(h1.finish(), h2.finish());
  }

  #[test]
  fn from_string_and_deref_mut() {
    let src = String::from("yo");
    let mut s: SmallString<4> = SmallString::from(src);
    assert_eq!(s.as_str(), "yo");
    s.make_ascii_uppercase();
    assert_eq!(s.as_str(), "YO");
  }

  #[test]
  fn from_str_constructor() {
    let s: SmallString<4> = SmallString::from_str("hey");
    assert_eq!(s.as_str(), "hey");
    assert!(s.is_inline());
  }

  #[cfg(feature = "serde")]
  mod serde_tests {
    use super::*;
    use serde_json;

    #[test]
    fn serialize_and_deserialize_string() {
      let s: SmallString<8> = SmallString::from("serde test");
      let json = serde_json::to_string(&s).unwrap();
      assert_eq!(json, "\"serde test\"");
      let de: SmallString<8> = serde_json::from_str(&json).unwrap();
      assert_eq!(de.as_str(), "serde test");
    }
  }
}
