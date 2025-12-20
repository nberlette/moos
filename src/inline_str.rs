use alloc::borrow::Borrow;
use alloc::borrow::BorrowMut;
use alloc::borrow::Cow;
use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::string::ToString;
use core::cmp::Ordering;
use core::convert::AsMut;
use core::convert::AsRef;
use core::convert::From;
use core::convert::TryFrom;
use core::fmt;
use core::fmt::Debug;
use core::fmt::Display;
use core::fmt::Formatter;
use core::hash::Hash;
use core::hash::Hasher;
use core::mem::size_of;
use core::ops::Deref;
use core::ops::DerefMut;
use core::str;
use core::str::FromStr;

use crate::CowStr;

/// Maximum length of an inline string in bytes. On 64-bit systems this is
/// typically 22 bytes, while on 32-bit systems, it's usually only 10 bytes.
///
/// This value is calculated as 3 times the size of an `isize` (to account for
/// UTF-8 encoding), **minus 2 bytes** to reserve space for a `u8` length byte
/// and a null terminator (`\0`) character (not stored but conceptually present
/// in a manner similar to C-style strings).
pub const MAX_INLINE_STR_LEN: usize = 3 * size_of::<isize>() - 2;

/// Error type returned when attempting to create an `InlineStr` from a string
/// or `&str` reference that exceeds the maximum allowed length determined by
/// the [`MAX_INLINE_STR_LEN`] constant.
///
/// # Example
///
/// ```rust
/// # use moos::inline_str::*;
/// # use core::convert::TryFrom;
/// # fn main() {
/// let long_str = "This string is too long to fit in an InlineStr";
/// let result = InlineStr::try_from(long_str);
///
/// assert!(result.is_err());
/// assert!(matches!(result, Err(StringTooLongError)));
///
/// # }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct StringTooLongError;

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "constructors", derive(derive_more::Constructor))]
#[cfg_attr(
  feature = "index",
  derive(derive_more::Index, derive_more::IndexMut)
)]
/// Represents a short inline string stored on the stack in fixed-size buffers.
///
/// Designed to hold very short strings (up to [`MAX_INLINE_STR_LEN`] bytes),
/// this type is useful for optimizing memory usage in scenarios where you
/// expect to frequently work with small strings.
///
/// Attempting to store a string longer than the maximum length will result in
/// a [`StringTooLongError`] being returned.
///
/// # Example
///
/// ```rust
/// # use moos::inline_str::*;
/// # use core::convert::TryFrom;
///
/// # fn main() -> Result<(), StringTooLongError> {
/// let inline_str: InlineStr = "Hello".parse()?;
/// assert_eq!(inline_str.as_ref(), "Hello");
/// assert_eq!(inline_str.len(), 5);
///
/// // This will fail because the string is too long:
/// let long_str = "This string is too long to fit in an InlineStr";
/// let result = InlineStr::try_from(long_str);
/// assert!(result.is_err());
/// assert!(matches!(result, Err(StringTooLongError)));
///
/// # Ok(())
/// # }
/// ```
pub struct InlineStr {
  #[cfg_attr(feature = "index", index)]
  #[cfg_attr(feature = "index", index_mut)]
  pub(crate) buf: [u8; MAX_INLINE_STR_LEN],
  pub(crate) len: u8,
}

impl InlineStr {
  /// Creates a new `InlineStr`.
  #[cfg(not(feature = "constructors"))]
  pub const fn new(buf: [u8; MAX_INLINE_STR_LEN], len: u8) -> Self {
    Self { buf, len }
  }

  /// Returns the length of the string.
  #[inline]
  pub const fn len(&self) -> usize {
    self.len as usize
  }

  /// Returns whether the string is empty.
  #[inline]
  pub const fn is_empty(&self) -> bool {
    self.len == 0
  }

  /// Returns a reference to the underlying byte buffer.
  #[inline]
  pub fn as_bytes(&self) -> &[u8] {
    &self.buf[..self.len as usize]
  }

  /// Returns a mutable reference to the underlying byte buffer.
  #[inline]
  pub fn as_bytes_mut(&mut self) -> &mut [u8] {
    &mut self.buf[..self.len as usize]
  }

  /// Returns a reference to the string as a slice.
  ///
  /// # Panics
  ///
  /// This method panics if the internal byte buffer does not contain valid
  /// UTF-8 data.
  #[inline]
  pub fn as_str(&self) -> &str {
    if let Ok(s) = str::from_utf8(self.as_bytes()) {
      s
    } else {
      panic!("InlineStr should only contain valid UTF-8 data");
    }
  }

  /// Returns a mutable reference to the string as a slice.
  #[inline]
  pub fn as_mut_str(&mut self) -> Result<&mut str, str::Utf8Error> {
    str::from_utf8_mut(self.as_bytes_mut())
  }

  /// Returns a reference to the string as a slice, without checking
  /// for UTF-8 validity.
  ///
  /// # Safety
  ///
  /// The caller must ensure the data is valid UTF-8.
  #[inline]
  pub unsafe fn as_str_unchecked(&self) -> &str {
    unsafe { str::from_utf8_unchecked(self.as_bytes()) }
  }

  /// Returns a mutable reference to the string as a slice, without checking
  /// for UTF-8 validity.
  ///
  /// # Safety
  ///
  /// The caller must ensure the data is valid UTF-8.
  #[inline]
  pub unsafe fn as_mut_str_unchecked(&mut self) -> &mut str {
    unsafe { str::from_utf8_unchecked_mut(self.as_bytes_mut()) }
  }
}

impl Default for InlineStr {
  #[inline(always)]
  fn default() -> Self {
    Self {
      buf: [0u8; MAX_INLINE_STR_LEN],
      len: 0,
    }
  }
}

impl Display for InlineStr {
  #[inline(always)]
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.as_str())
  }
}

impl Borrow<str> for InlineStr {
  #[inline(always)]
  fn borrow(&self) -> &str {
    self.as_ref()
  }
}

impl BorrowMut<str> for InlineStr {
  #[inline(always)]
  fn borrow_mut(&mut self) -> &mut str {
    self.as_mut_str().unwrap_or_default()
  }
}

impl Deref for InlineStr {
  type Target = str;

  #[inline(always)]
  fn deref(&self) -> &str {
    self.as_str()
  }
}

impl DerefMut for InlineStr {
  #[inline(always)]
  fn deref_mut(&mut self) -> &mut str {
    self.as_mut_str().unwrap_or_default()
  }
}

impl AsRef<str> for InlineStr {
  #[inline(always)]
  fn as_ref(&self) -> &str {
    self.deref()
  }
}

impl AsMut<str> for InlineStr {
  #[inline(always)]
  fn as_mut(&mut self) -> &mut str {
    self.deref_mut()
  }
}

impl From<InlineStr> for String {
  #[inline(always)]
  fn from(s: InlineStr) -> Self {
    s.deref().to_owned()
  }
}

impl From<&InlineStr> for String {
  #[inline(always)]
  fn from(s: &InlineStr) -> Self {
    s.deref().to_owned()
  }
}

impl<T: AsRef<str>> From<&T> for InlineStr {
  #[inline(always)]
  fn from(s: &T) -> Self {
    InlineStr::try_from(s.as_ref())
      .expect("String length exceeds InlineStr maximum capacity")
  }
}

impl From<char> for InlineStr {
  #[inline(always)]
  fn from(c: char) -> Self {
    let mut buf = [0u8; MAX_INLINE_STR_LEN];
    c.encode_utf8(&mut buf);
    let len = c.len_utf8() as u8;
    Self { buf, len }
  }
}

impl<'i> From<Cow<'i, str>> for InlineStr {
  #[inline(always)]
  fn from(cow: Cow<'i, str>) -> Self {
    let src = cow.as_ref().as_bytes();
    let len = src.len().min(MAX_INLINE_STR_LEN);
    let mut buf = [0u8; MAX_INLINE_STR_LEN];
    buf[..len].copy_from_slice(&src[..len]);
    let len = len as u8;
    Self { buf, len }
  }
}

impl FromStr for InlineStr {
  type Err = StringTooLongError;

  #[inline(always)]
  fn from_str(s: &str) -> Result<InlineStr, StringTooLongError> {
    InlineStr::try_from(s)
  }
}

impl From<String> for InlineStr {
  #[inline(always)]
  fn from(s: String) -> Self {
    let src = s.as_bytes();
    let len = src.len().min(MAX_INLINE_STR_LEN);
    let mut buf = [0u8; MAX_INLINE_STR_LEN];
    buf[..len].copy_from_slice(&src[..len]);
    let len = len as u8;
    Self { buf, len }
  }
}

impl TryFrom<&str> for InlineStr {
  type Error = StringTooLongError;

  #[inline(always)]
  fn try_from(s: &str) -> Result<InlineStr, StringTooLongError> {
    let len = s.len();
    if len > MAX_INLINE_STR_LEN {
      return Err(StringTooLongError);
    }
    let mut buf = [0u8; MAX_INLINE_STR_LEN];
    buf[..len].copy_from_slice(s.as_bytes());
    let len = len as u8;
    Ok(Self { buf, len })
  }
}

impl Hash for InlineStr {
  #[inline(always)]
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.deref().hash(state);
  }
}

impl<T: ToString> PartialEq<T> for InlineStr {
  #[inline(always)]
  fn eq(&self, other: &T) -> bool {
    self.deref() == other.to_string()
  }
}

impl PartialEq<InlineStr> for &InlineStr {
  #[inline(always)]
  fn eq(&self, other: &InlineStr) -> bool {
    **self == *other
  }
}

impl PartialEq<str> for InlineStr {
  #[inline(always)]
  fn eq(&self, other: &str) -> bool {
    self.deref() == other
  }
}

impl<'i> PartialEq<InlineStr> for Cow<'i, str> {
  #[inline(always)]
  fn eq(&self, other: &InlineStr) -> bool {
    self.deref() == other.deref()
  }
}

impl<'i> PartialEq<InlineStr> for CowStr<'i> {
  #[inline(always)]
  fn eq(&self, other: &InlineStr) -> bool {
    self.deref() == other.deref()
  }
}

impl PartialEq<InlineStr> for &str {
  #[inline(always)]
  fn eq(&self, other: &InlineStr) -> bool {
    *self == other.deref()
  }
}

impl PartialEq<InlineStr> for str {
  #[inline(always)]
  fn eq(&self, other: &InlineStr) -> bool {
    self == other.deref()
  }
}

impl PartialEq<InlineStr> for char {
  #[inline(always)]
  fn eq(&self, other: &InlineStr) -> bool {
    let other_str = other.deref();
    if let Some(first_char) = other_str.chars().next() {
      first_char == *self && other_str.len() == self.len_utf8()
    } else {
      false
    }
  }
}

impl PartialEq<InlineStr> for String {
  #[inline(always)]
  fn eq(&self, other: &InlineStr) -> bool {
    self.as_str() == other.deref()
  }
}

impl PartialEq<InlineStr> for &String {
  #[inline(always)]
  fn eq(&self, other: &InlineStr) -> bool {
    self.as_str() == other.deref()
  }
}

impl PartialEq<InlineStr> for &&str {
  #[inline(always)]
  fn eq(&self, other: &InlineStr) -> bool {
    **self == other.deref()
  }
}

impl PartialEq<InlineStr> for &mut str {
  #[inline(always)]
  fn eq(&self, other: &InlineStr) -> bool {
    &**self == other.deref()
  }
}

impl PartialEq<InlineStr> for &mut String {
  #[inline(always)]
  fn eq(&self, other: &InlineStr) -> bool {
    self.as_str() == other.deref()
  }
}

impl PartialEq<InlineStr> for &mut InlineStr {
  #[inline(always)]
  fn eq(&self, other: &InlineStr) -> bool {
    **self == *other
  }
}

impl Eq for InlineStr {}

impl PartialOrd<str> for InlineStr {
  #[inline(always)]
  fn partial_cmp(&self, other: &str) -> Option<Ordering> {
    Some(self.deref().cmp(other))
  }
}

impl PartialOrd<InlineStr> for str {
  #[inline(always)]
  fn partial_cmp(&self, other: &InlineStr) -> Option<Ordering> {
    Some(self.cmp(other.deref()))
  }
}

impl PartialOrd<InlineStr> for char {
  #[inline(always)]
  fn partial_cmp(&self, other: &InlineStr) -> Option<Ordering> {
    let that = other.deref();
    if let Some(first_char) = that.chars().next() {
      Some(self.cmp(&first_char))
    } else {
      Some(Ordering::Greater)
    }
  }
}

impl PartialOrd<InlineStr> for String {
  fn partial_cmp(&self, other: &InlineStr) -> Option<Ordering> {
    Some(self.as_str().cmp(other.deref()))
  }
}

impl PartialOrd<InlineStr> for &String {
  #[inline(always)]
  fn partial_cmp(&self, other: &InlineStr) -> Option<Ordering> {
    Some(self.as_str().cmp(other.deref()))
  }
}

impl PartialOrd<InlineStr> for &&str {
  #[inline(always)]
  fn partial_cmp(&self, other: &InlineStr) -> Option<Ordering> {
    Some((**self).cmp(other.deref()))
  }
}

impl PartialOrd<InlineStr> for &mut str {
  #[inline(always)]
  fn partial_cmp(&self, other: &InlineStr) -> Option<Ordering> {
    Some((**self).cmp(other.deref()))
  }
}

impl PartialOrd<InlineStr> for &mut String {
  #[inline(always)]
  fn partial_cmp(&self, other: &InlineStr) -> Option<Ordering> {
    Some(self.as_str().cmp(other.deref()))
  }
}

impl PartialOrd<InlineStr> for &mut InlineStr {
  #[inline(always)]
  fn partial_cmp(&self, other: &InlineStr) -> Option<Ordering> {
    Some((**self).deref().cmp(other.deref()))
  }
}

impl<'i> PartialOrd<InlineStr> for Cow<'i, str> {
  #[inline(always)]
  fn partial_cmp(&self, other: &InlineStr) -> Option<Ordering> {
    Some(self.deref().cmp(other.deref()))
  }
}

impl<'i> PartialOrd<InlineStr> for CowStr<'i> {
  #[inline(always)]
  fn partial_cmp(&self, other: &InlineStr) -> Option<Ordering> {
    Some(self.deref().cmp(other.deref()))
  }
}

impl<T: ToString> PartialOrd<T> for InlineStr {
  #[inline(always)]
  fn partial_cmp(&self, other: &T) -> Option<Ordering> {
    let that = other.to_string();
    Some(self.deref().cmp(&that))
  }
}

impl Ord for InlineStr {
  #[inline(always)]
  fn cmp(&self, other: &Self) -> Ordering {
    self.deref().cmp(other.deref())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn max_inline_str_len_is_at_least_4_bytes() {
    assert!(MAX_INLINE_STR_LEN >= 4);
  }

  #[test]
  fn inline_str_from_ascii_char() {
    let s: InlineStr = 'a'.into();
    assert_eq!("a", s.deref());
  }

  #[test]
  fn inline_str_from_unicode_char() {
    let s: InlineStr = '🍔'.into();
    assert_eq!("🍔", s.deref());
  }

  #[test]
  #[cfg(target_pointer_width = "64")]
  fn inline_str_fits_twentytwo() {
    let s = "0123456789abcdefghijkl";
    let stack_str = InlineStr::try_from(s);
    assert!(stack_str.is_ok());
    let stack_str = stack_str.unwrap();
    assert_eq!(stack_str.len(), 22);
    assert_eq!(stack_str.deref().len(), 22);
    assert_eq!(stack_str.deref(), s);
  }

  #[test]
  #[cfg(target_pointer_width = "64")]
  fn inline_str_not_fits_twentythree() {
    let s = "0123456789abcdefghijklm";
    let err = InlineStr::try_from(s);
    assert!(err.is_err());
    assert!(matches!(err, Err(StringTooLongError)));
  }

  #[test]
  #[cfg(target_pointer_width = "64")]
  fn try_inline_str_from_str() {
    let s = "Hello, world!";
    let inline_str = InlineStr::try_from(s);
    assert!(inline_str.is_ok());
    let inline_str = inline_str.unwrap();
    assert_eq!(inline_str.deref(), s);
  }

  #[test]
  #[cfg(target_pointer_width = "32")]
  fn inline_str_fits_ten() {
    let s = "0123456789";
    let stack_str = InlineStr::try_from(s);
    assert!(stack_str.is_ok());
    let stack_str = stack_str.unwrap();
    assert_eq!(stack_str.len(), 10);
    assert_eq!(stack_str.deref().len(), 10);
    assert_eq!(stack_str.deref(), s);
  }

  #[test]
  #[cfg(target_pointer_width = "32")]
  fn inline_str_not_fits_eleven() {
    let s = "0123456789a";
    let err = InlineStr::try_from(s);
    assert!(err.is_err());
    assert!(matches!(err, Err(StringTooLongError)));
  }

  #[test]
  fn try_inline_str_from_long_str() {
    let s = "This string is too long to fit in an InlineStr";
    let err = InlineStr::try_from(s);
    assert!(err.is_err());
    assert!(matches!(err, Err(StringTooLongError)));
  }

  #[test]
  fn inline_str_equality() {
    let s1: InlineStr = "Hello".try_into().unwrap();
    let s2: InlineStr = "Hello".try_into().unwrap();
    let s3: InlineStr = "World".try_into().unwrap();
    assert_eq!(s1, s2);
    assert_ne!(s1, s3);
    assert!(s1 < s3);
    assert!(s2 <= s1);
    assert!(s3 > s1);
  }

  #[test]
  fn inline_str_char_equality() {
    let s: InlineStr = "A".try_into().unwrap();
    let c: char = 'A';
    assert_eq!(s, c);
    assert_eq!(c, s);
  }

  #[test]
  fn inline_str_cow_equality() {
    let s: InlineStr = "Hello".try_into().unwrap();
    let cow: Cow<str> = Cow::Borrowed("Hello");
    assert_eq!(s, cow);
    assert_eq!(cow, s);
  }

  #[test]
  fn inline_str_as_mut_str() {
    let mut s: InlineStr = "Hello".try_into().unwrap();
    {
      let s_mut = s.as_mut_str().unwrap();
      s_mut.make_ascii_uppercase();
      assert_eq!(s_mut, "HELLO");
    }
    assert_eq!(s, "HELLO");
  }
}
