use alloc::borrow::Borrow;
use alloc::borrow::BorrowMut;
use alloc::borrow::Cow;
use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::string::ToString;
use core::convert::AsMut;
use core::convert::AsRef;
use core::convert::From;
use core::convert::Into;
use core::fmt;
use core::fmt::Display;
use core::hash::Hash;
use core::hash::Hasher;
use core::mem::transmute_copy;
use core::ops::Deref;
use core::ops::DerefMut;
use core::str;

use crate::inline_str::*;

/// Copy-on-write string that can be owned, borrowed, or inlined.
///
/// # Variants
///
/// 1. [`Owned`](CowStr::Owned): Boxed string slice that owns the data. No
///    lifetime parameter is needed here, since the data is owned by the
///    `CowStr` instance itself.
/// 2. [`Borrowed`](CowStr::Borrowed): Borrowed string slice. Does not own the
///    data, so it must specify the lifetime parameter `'i` to indicate how long
///    the data will live for.
/// 3. [`Inlined`](CowStr::Inlined): Short inline string stored on the stack
///    using the [`InlineStr`] type. Must be [`MAX_INLINE_STR_LEN`] bytes or
///    less in length (typically 22 bytes on 64-bit systems).
///
/// # Examples
///
/// ```rust
/// # use moos::CowStr;
///
/// # fn main() -> Result<(), moos::inline_str::StringTooLongError> {
/// let owned = CowStr::Owned("This is an owned string.".into());
/// // this is a fallible conversion, thus `From<&str>` is not implemented.
/// let inlined = CowStr::Inlined("smol str!".parse()?);
/// let borrowed = CowStr::Borrowed("This is a borrowed string.");
///
/// // checking if a CowStr is inlined, owned, or borrowed
/// assert!(owned.is_owned(), "Expected an owned CowStr!");
/// assert!(inlined.is_inlined(), "Expected an inlined CowStr!");
/// assert!(borrowed.is_borrowed(), "Expected a borrowed CowStr!");
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Eq)]
#[cfg_attr(feature = "is_variant", derive(derive_more::IsVariant))]
pub enum CowStr<'i> {
  /// An immutable boxed string slice that owns the data. This is the
  /// default variant for owned strings (i.e. [`String`] instances), which
  /// are always stored on the heap.
  Owned(Box<str>),
  /// A short inline string stored on the stack using [`InlineStr`].
  ///
  /// This is useful for optimizing memory usage in scenarios where you
  /// expect to frequently work with small strings. Only supports string
  /// lengths up to [`MAX_INLINE_STR_LEN`].
  Inlined(InlineStr),
  /// A borrowed string slice that does not own the data. This is the
  /// default variant for borrowed `&str` references, which are stored on
  /// the stack in most cases. Must specify the lifetime parameter `'i` to
  /// indicate the lifetime of the data being borrowed.
  Borrowed(&'i str),
}

impl<'i> CowStr<'i> {
  #[inline(always)]
  pub fn as_str(&self) -> &str {
    match self {
      CowStr::Owned(b) => b,
      CowStr::Borrowed(b) => b,
      CowStr::Inlined(s) => s.deref(),
    }
  }

  /// Returns a mutable reference to the string as a slice.
  ///
  /// # Safety
  ///
  /// The caller must ensure that the mutable reference does not violate any
  /// aliasing rules, i.e., there are no other references to the same data while
  /// this mutable reference is in use. This is especially important for the
  /// `Borrowed` variant, as modifying the data could lead to undefined behavior
  /// if there are other references to the same data. Use with caution and
  /// discretion.
  #[inline(always)]
  pub unsafe fn as_mut_str(&mut self) -> &mut str {
    unsafe {
      match self {
        CowStr::Owned(b) => b,
        CowStr::Borrowed(b) => transmute_copy(&b.to_owned().as_bytes_mut()),
        CowStr::Inlined(s) => s.as_mut_str_unchecked(),
      }
    }
  }

  #[inline(always)]
  pub fn as_bytes(&self) -> &[u8] {
    match self {
      CowStr::Owned(b) => b.as_bytes(),
      CowStr::Borrowed(b) => b.as_bytes(),
      CowStr::Inlined(s) => s.as_bytes(),
    }
  }

  /// Returns a mutable byte slice of the string's contents.
  ///
  /// # Safety
  ///
  /// The caller must ensure that the underlying data is not aliased while the
  /// mutable byte slice is in use. This is particularly important for the
  /// [`CowStr::Borrowed`] variant - modifying the data while there are existing
  /// references to it is undefined behavior. Use with caution.
  #[inline(always)]
  pub unsafe fn as_bytes_mut(&mut self) -> &mut [u8] {
    unsafe {
      match *self {
        CowStr::Owned(ref mut b) => b.as_bytes_mut(),
        CowStr::Borrowed(b) => transmute_copy(&b.to_owned().as_bytes_mut()),
        CowStr::Inlined(ref mut s) => s.as_bytes_mut(),
      }
    }
  }

  #[inline(always)]
  pub fn len(&self) -> usize {
    self.as_bytes().len()
  }

  #[inline(always)]
  pub fn into_owned(self) -> String {
    match self {
      CowStr::Owned(s) => s.into(),
      CowStr::Borrowed(s) => s.to_owned(),
      CowStr::Inlined(s) => s.deref().to_owned(),
    }
  }

  #[inline(always)]
  pub fn into_string(self) -> String {
    match self {
      CowStr::Owned(b) => b.into(),
      CowStr::Borrowed(b) => b.to_owned(),
      CowStr::Inlined(s) => s.deref().to_owned(),
    }
  }
}

impl<'i> Display for CowStr<'i> {
  #[inline(always)]
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.deref())
  }
}

impl<'i> Default for CowStr<'i> {
  #[inline(always)]
  fn default() -> Self {
    CowStr::Borrowed("")
  }
}

impl<'i> Hash for CowStr<'i> {
  #[inline(always)]
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.deref().hash(state);
  }
}

impl<'i> Clone for CowStr<'i> {
  #[inline]
  fn clone(&self) -> Self {
    match self {
      CowStr::Owned(s) => match InlineStr::try_from(&**s) {
        Ok(inline) => CowStr::Inlined(inline),
        Err(_) => CowStr::Owned(s.clone()),
      },
      CowStr::Borrowed(s) => CowStr::Borrowed(s),
      CowStr::Inlined(s) => CowStr::Inlined(*s),
    }
  }
}

impl<'i> Deref for CowStr<'i> {
  type Target = str;

  #[inline(always)]
  fn deref(&self) -> &Self::Target {
    self.as_str()
  }
}

impl<'i> DerefMut for CowStr<'i> {
  #[inline(always)]
  fn deref_mut(&mut self) -> &mut str {
    unsafe { self.as_mut_str() }
  }
}

impl<'i> AsRef<str> for CowStr<'i> {
  #[inline(always)]
  fn as_ref(&self) -> &str {
    self.deref()
  }
}

impl<'i> AsMut<str> for CowStr<'i> {
  #[inline(always)]
  fn as_mut(&mut self) -> &mut str {
    self.deref_mut()
  }
}

impl<'i> Borrow<str> for CowStr<'i> {
  fn borrow(&self) -> &str {
    self.deref()
  }
}

impl<'i> BorrowMut<str> for CowStr<'i> {
  fn borrow_mut(&mut self) -> &mut str {
    self.deref_mut()
  }
}

impl<'i> PartialEq for CowStr<'i> {
  #[inline(always)]
  fn eq(&self, other: &Self) -> bool {
    self.deref() == other.deref()
  }
}

impl<'i> PartialEq<str> for CowStr<'i> {
  #[inline(always)]
  fn eq(&self, other: &str) -> bool {
    self.deref() == other
  }
}

impl<'i> PartialEq<&'i str> for CowStr<'i> {
  #[inline(always)]
  fn eq(&self, other: &&'i str) -> bool {
    self.deref() == *other
  }
}

impl<'i> PartialEq<Cow<'i, str>> for CowStr<'i> {
  #[inline(always)]
  fn eq(&self, other: &Cow<'i, str>) -> bool {
    self.deref() == other.deref()
  }
}

impl<'i> PartialEq<CowStr<'i>> for str {
  #[inline(always)]
  fn eq(&self, other: &CowStr<'_>) -> bool {
    self == other.deref()
  }
}

impl<'i> PartialEq<CowStr<'i>> for &'i str {
  #[inline(always)]
  fn eq(&self, other: &CowStr<'_>) -> bool {
    other.deref() == *self
  }
}

impl<'i> PartialEq<CowStr<'i>> for Cow<'i, str> {
  #[inline(always)]
  fn eq(&self, other: &CowStr<'_>) -> bool {
    self.deref() == other.deref()
  }
}

impl<'i> PartialEq<String> for CowStr<'i> {
  #[inline(always)]
  fn eq(&self, other: &String) -> bool {
    self.deref() == other.deref()
  }
}

impl<'i> PartialEq<CowStr<'i>> for String {
  #[inline(always)]
  fn eq(&self, other: &CowStr<'_>) -> bool {
    self.deref() == other.deref()
  }
}

impl<'i> PartialOrd<CowStr<'i>> for CowStr<'i> {
  #[inline(always)]
  fn partial_cmp(&self, other: &CowStr<'_>) -> Option<core::cmp::Ordering> {
    self.deref().partial_cmp(other.deref())
  }
}

impl<'i> PartialOrd<str> for CowStr<'i> {
  #[inline(always)]
  fn partial_cmp(&self, other: &str) -> Option<core::cmp::Ordering> {
    self.deref().partial_cmp(other)
  }
}

impl<'i> PartialOrd<&'i str> for CowStr<'i> {
  #[inline(always)]
  fn partial_cmp(&self, other: &&'i str) -> Option<core::cmp::Ordering> {
    self.deref().partial_cmp(*other)
  }
}

impl<'i> PartialOrd<Cow<'i, str>> for CowStr<'i> {
  #[inline(always)]
  fn partial_cmp(&self, other: &Cow<'i, str>) -> Option<core::cmp::Ordering> {
    self.deref().partial_cmp(other.deref())
  }
}

impl<'i> PartialOrd<CowStr<'i>> for str {
  #[inline(always)]
  fn partial_cmp(&self, other: &CowStr<'_>) -> Option<core::cmp::Ordering> {
    self.partial_cmp(other.deref())
  }
}

impl<'i> From<&'i str> for CowStr<'i> {
  #[inline(always)]
  fn from(s: &'i str) -> Self {
    CowStr::Borrowed(s)
  }
}

impl<'i> From<String> for CowStr<'i> {
  #[inline(always)]
  fn from(s: String) -> Self {
    CowStr::Owned(s.into_boxed_str())
  }
}

impl<'i> From<char> for CowStr<'i> {
  #[inline(always)]
  fn from(c: char) -> Self {
    CowStr::Inlined(c.into())
  }
}

impl<'i> From<Cow<'i, str>> for CowStr<'i> {
  #[inline(always)]
  fn from(s: Cow<'i, str>) -> Self {
    match s {
      Cow::Borrowed(s) => CowStr::Borrowed(s),
      Cow::Owned(s) => CowStr::Owned(s.into_boxed_str()),
    }
  }
}

impl<'i> From<CowStr<'i>> for Cow<'i, str> {
  #[inline(always)]
  fn from(s: CowStr<'i>) -> Self {
    match s {
      CowStr::Owned(s) => Cow::Owned(s.to_string()),
      CowStr::Inlined(s) => Cow::Owned(s.to_string()),
      CowStr::Borrowed(s) => Cow::Borrowed(s),
    }
  }
}

impl<'i> From<Cow<'i, char>> for CowStr<'i> {
  #[inline(always)]
  fn from(s: Cow<'i, char>) -> Self {
    CowStr::Inlined(InlineStr::from(*s.deref()))
  }
}

impl<'i> From<CowStr<'i>> for String {
  #[inline(always)]
  fn from(s: CowStr<'i>) -> Self {
    s.into_string()
  }
}

#[cfg(feature = "serde")]
mod serde_impl {
  use core::fmt;

  use serde::Deserialize;
  use serde::Deserializer;
  use serde::Serialize;
  use serde::Serializer;
  use serde::de;

  use super::*;

  impl<'i> Serialize for CowStr<'i> {
    #[inline(always)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
      S: Serializer,
    {
      serializer.serialize_str(self.as_ref())
    }
  }

  struct CowStrVisitor;

  impl<'de> de::Visitor<'de> for CowStrVisitor {
    type Value = CowStr<'de>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
      formatter.write_str("a string")
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
      E: de::Error,
    {
      Ok(CowStr::Borrowed(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
      E: de::Error,
    {
      match v.try_into() {
        Ok(it) => Ok(CowStr::Inlined(it)),
        Err(_) => Ok(CowStr::Owned(String::from(v).into_boxed_str())),
      }
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
      E: de::Error,
    {
      Ok(CowStr::Owned(v.into_boxed_str()))
    }
  }

  impl<'i, 'de: 'i> Deserialize<'de> for CowStr<'i> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
      D: Deserializer<'de>,
    {
      deserializer.deserialize_str(CowStrVisitor)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn cowstr_size() {
    let size = std::mem::size_of::<CowStr>();
    let word_size = std::mem::size_of::<isize>();
    assert_eq!(3 * word_size, size);
  }

  #[test]
  fn cowstr_char_to_string() {
    let c = '藏';
    let smort: CowStr = c.into();
    let owned: String = smort.to_string();
    let expected = "藏".to_owned();
    assert_eq!(expected, owned);
  }

  #[test]
  #[cfg(target_pointer_width = "64")]
  fn small_boxed_str_clones_to_stack() {
    let s = "0123456789abcde".to_owned();
    let smort: CowStr = s.into();
    let smort_clone = smort.clone();

    if let CowStr::Inlined(..) = smort_clone {
    } else {
      panic!("Expected a Inlined variant!");
    }
  }

  #[test]
  fn cow_to_cow_str() {
    let s = "some text";
    let cow = Cow::Borrowed(s);
    let actual = CowStr::from(cow);
    let expected = CowStr::Borrowed(s);
    assert_eq!(actual, expected);
    assert!(variant_eq(&actual, &expected));

    let s = "some text".to_string();
    let cow: Cow<str> = Cow::Owned(s.clone());
    let actual = CowStr::from(cow);
    let expected = CowStr::Owned(s.into_boxed_str());
    assert_eq!(actual, expected);
    assert!(variant_eq(&actual, &expected));
  }

  #[test]
  fn cow_str_to_cow() {
    let s = "some text";
    let cow_str = CowStr::Borrowed(s);
    let actual = Cow::from(cow_str);
    let expected = Cow::Borrowed(s);
    assert_eq!(actual, expected);
    assert!(variant_eq(&actual, &expected));

    let s = "s";
    let inline_str: InlineStr = InlineStr::try_from(s).unwrap();
    let cow_str = CowStr::Inlined(inline_str);
    let actual = Cow::from(cow_str);
    let expected: Cow<str> = Cow::Owned(s.to_string());
    assert_eq!(actual, expected);
    assert!(variant_eq(&actual, &expected));

    let s = "s";
    let cow_str = CowStr::Owned(s.to_string().into_boxed_str());
    let actual = Cow::from(cow_str);
    let expected: Cow<str> = Cow::Owned(s.to_string());
    assert_eq!(actual, expected);
    assert!(variant_eq(&actual, &expected));
  }

  #[test]
  fn cow_str_to_string() {
    let s = "some text";
    let cow_str = CowStr::Borrowed(s);
    let actual = String::from(cow_str);
    let expected = String::from("some text");
    assert_eq!(actual, expected);

    let s = "s";
    let inline_str: InlineStr = InlineStr::try_from(s).unwrap();
    let cow_str = CowStr::Inlined(inline_str);
    let actual = String::from(cow_str);
    let expected = String::from("s");
    assert_eq!(actual, expected);

    let s = "s";
    let cow_str = CowStr::Owned(s.to_string().into_boxed_str());
    let actual = String::from(cow_str);
    let expected = String::from("s");
    assert_eq!(actual, expected);
  }

  #[test]
  fn cow_char_to_cow_str() {
    let c = 'c';
    let cow: Cow<char> = Cow::Owned(c);
    let actual = CowStr::from(cow);
    let expected = CowStr::Inlined(InlineStr::from(c));
    assert_eq!(actual, expected);
    assert!(variant_eq(&actual, &expected));

    let c = 'c';
    let cow: Cow<char> = Cow::Borrowed(&c);
    let actual = CowStr::from(cow);
    let expected = CowStr::Inlined(InlineStr::from(c));
    assert_eq!(actual, expected);
    assert!(variant_eq(&actual, &expected));
  }

  fn variant_eq<T>(a: &T, b: &T) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
  }
}
