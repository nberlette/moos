//! An inline‑optimized vector type.
//!
//! `CompactVec<T, N>` is a vector that stores up to `N` elements in
//! preallocated inline storage and transparently spills over to a
//! heap‑allocated `Vec<T>` when more capacity is required. When the
//! optional `serde` feature is enabled, it implements `Serialize` and
//! `Deserialize` so you can easily persist its contents.
//!
//! ## Examples
//!
//! Creating a `CompactVec` with four inline slots and pushing a few
//! elements into it:
//!
//! ```
//! use moos::CompactVec;
//!
//! // this instance will store up to four elements on the stack
//! let mut vec: CompactVec<u32, 4> = CompactVec::new();
//! for i in 0..3 {
//!   vec.push(i);
//! }
//! assert_eq!(vec.len(), 3);
//! assert_eq!(vec.as_slice(), &[0, 1, 2][..]);
//! assert!(vec.is_inline());
//!
//! // pushing beyond the inline capacity causes it to spill to the heap
//! vec.push(3);
//! vec.push(4);
//! assert!(!vec.is_inline());
//! assert_eq!(vec.as_slice(), &[0, 1, 2, 3, 4]);
//! ```
//!
//! ### Serde
//!
//! If compiled with the `serde` feature, `CompactVec` implements
//! [`serde::Serialize`] and [`serde::Deserialize`]. These impls
//! serialize the vector as a regular sequence and deserialize from any
//! compatible sequence. When deserializing, the inline capacity is used
//! to minimize allocations whenever possible.

use core::fmt;
use core::iter::FromIterator;
use core::iter::IntoIterator;
use core::mem::MaybeUninit;
use core::ops::Deref;
use core::ops::DerefMut;
use core::ops::Index;
use core::ops::IndexMut;

use alloc::vec::Vec;

/// A vector type that stores up to `N` elements inline before spilling
/// to a heap allocation.
///
/// The generic parameter `N` controls the number of inline slots. When
/// the length of the vector is less than or equal to `N`, the values are
/// stored directly within the structure itself, avoiding any heap
/// allocations. Once more than `N` elements are pushed, the existing
/// inline elements are moved into a `Vec<T>` and all subsequent pushes
/// append to that `Vec`.
///
/// # Safety
///
/// This type uses `MaybeUninit<T>` internally to manage the inline
/// storage. Care is taken to correctly initialize and drop elements, but
/// misuse of unsafe code could lead to undefined behavior. The public
/// API of `CompactVec` should be safe to use; unsafe blocks are only
/// employed internally to implement functionality that cannot be
/// expressed safely in stable Rust today.
pub struct CompactVec<T, const N: usize> {
  /// Inline storage for up to `N` elements. Elements are written into
  /// this array until it is full. Once full, the data is moved into
  /// the `heap` vector and this array is left uninitialized.
  inline: [MaybeUninit<T>; N],
  /// The current number of initialized elements in the `inline`
  /// storage. This field is only meaningful when `heap` is `None`.
  len:    usize,
  /// Heap storage used when more than `N` elements are present. When
  /// `Some`, all elements live in this vector and `inline` should be
  /// considered uninitialized.
  heap:   Option<Vec<T>>,
}

impl<T, const N: usize> CompactVec<T, N> {
  /// Creates a new empty `CompactVec` with the specified inline
  /// capacity. No heap allocation occurs until more than `N` elements
  /// are pushed.
  pub fn new() -> Self {
    // SAFETY: An uninitialized array of `MaybeUninit<T>` is valid.
    let inline =
      unsafe { MaybeUninit::<[MaybeUninit<T>; N]>::uninit().assume_init() };
    Self {
      inline,
      len: 0,
      heap: None,
    }
  }

  /// Creates a new `CompactVec` with enough capacity to hold at least
  /// `capacity` elements without reallocation. If `capacity` is less
  /// than or equal to `N`, no heap allocation will occur. Otherwise,
  /// a `Vec` of the requested capacity will be allocated.
  pub fn with_capacity(capacity: usize) -> Self {
    if capacity <= N {
      Self::new()
    } else {
      Self {
        // SAFETY: uninitialized array is valid for inline storage.
        inline: unsafe {
          MaybeUninit::<[MaybeUninit<T>; N]>::uninit().assume_init()
        },
        len:    0,
        heap:   Some(Vec::with_capacity(capacity)),
      }
    }
  }

  /// Returns the number of elements in the vector.
  pub const fn len(&self) -> usize {
    match &self.heap {
      Some(heap) => heap.len(),
      None => self.len,
    }
  }

  /// Returns `true` if the vector contains no elements.
  pub const fn is_empty(&self) -> bool {
    self.len() == 0
  }

  /// Returns the total capacity of the vector. When stored inline,
  /// this equals `N`; when stored on the heap, this delegates to the
  /// internal `Vec`'s capacity.
  pub fn capacity(&self) -> usize {
    match &self.heap {
      Some(heap) => heap.capacity(),
      None => N,
    }
  }

  /// Returns `true` if the data is currently stored inline (i.e.,
  /// `len() <= N` and `heap` is `None`).
  pub fn is_inline(&self) -> bool {
    self.heap.is_none()
  }

  /// Provides an immutable slice of all elements in the vector.
  pub const fn as_slice(&self) -> &[T] {
    match &self.heap {
      Some(heap) => heap.as_slice(),
      None => {
        // SAFETY: The first `len` elements of `inline` are
        // initialized. We create a slice of that many elements.
        unsafe {
          core::slice::from_raw_parts(
            self.inline.as_ptr() as *const T,
            self.len,
          )
        }
      }
    }
  }

  /// Provides a mutable slice of all elements in the vector.
  pub const fn as_mut_slice(&mut self) -> &mut [T] {
    match &mut self.heap {
      Some(heap) => heap.as_mut_slice(),
      None => {
        // SAFETY: The first `len` elements of `inline` are
        // initialized. We create a mutable slice of that many
        // elements. No aliasing occurs because either `heap` is
        // `None` (so no other references exist) or we go into the
        // `Some` branch above.
        unsafe {
          core::slice::from_raw_parts_mut(
            self.inline.as_mut_ptr() as *mut T,
            self.len,
          )
        }
      }
    }
  }

  /// Pushes a value onto the end of the vector. If the inline
  /// storage is full, all existing elements are moved into a new
  /// `Vec` and subsequent pushes are delegated to that vector.
  pub fn push(&mut self, value: T) {
    match self.heap {
      Some(ref mut heap) => {
        heap.push(value);
      }
      None => {
        if self.len < N {
          // SAFETY: We have capacity in `inline` at index `len`.
          unsafe {
            self.inline[self.len].as_mut_ptr().write(value);
          }
          self.len += 1;
        } else {
          // Spill to heap: allocate a new Vec with double the
          // previous capacity for amortized growth.
          let mut vec = Vec::with_capacity(N * 2 + 1);
          // Move the existing inline elements into the Vec.
          for i in 0..self.len {
            // SAFETY: `i < len` so inline[i] is initialized.
            unsafe {
              vec.push(self.inline[i].assume_init_read());
            }
          }
          vec.push(value);
          self.heap = Some(vec);
          // We no longer use the inline storage, so reset len.
          self.len = 0;
        }
      }
    }
  }

  /// Removes the last element from the vector and returns it, or
  /// `None` if it is empty. If popping from a heap‑backed vector
  /// results in a length that can be stored inline, the data is
  /// automatically moved back into the inline storage to free the
  /// heap allocation.
  pub fn pop(&mut self) -> Option<T> {
    match self.heap {
      Some(ref mut heap) => {
        let value = heap.pop();
        if let Some(v) = value {
          // If the remaining length fits into inline storage, move
          // back onto the stack.
          if heap.len() <= N {
            let mut new_len = 0;
            for elem in heap.drain(..) {
              // SAFETY: We have ensured that `heap.len()`
              // is less than or equal to `N`, so there is
              // enough space in `inline` to store all
              // remaining elements.
              unsafe {
                self.inline[new_len].as_mut_ptr().write(elem);
              }
              new_len += 1;
            }
            self.heap = None;
            self.len = new_len;
          }
          Some(v)
        } else {
          None
        }
      }
      None => {
        if self.len == 0 {
          None
        } else {
          self.len -= 1;
          // SAFETY: `len` has been decremented, so the element
          // at index `len` is initialized and can be read. After
          // reading, we leave the memory uninitialized.
          Some(unsafe { self.inline[self.len].assume_init_read() })
        }
      }
    }
  }

  /// Clears the vector, removing all values. This resets the vector
  /// back to an empty inline state, deallocating any heap storage.
  pub fn clear(&mut self) {
    match self.heap {
      Some(ref mut heap) => {
        heap.clear();
        self.heap = None;
        self.len = 0;
      }
      None => {
        // Drop all inline elements
        for i in 0..self.len {
          unsafe {
            self.inline[i].assume_init_drop();
          }
        }
        self.len = 0;
      }
    }
  }

  /// Returns an iterator over the slice.
  pub fn iter(&self) -> core::slice::Iter<'_, T> {
    self.as_slice().iter()
  }

  /// Returns a mutable iterator over the slice.
  pub fn iter_mut(&mut self) -> core::slice::IterMut<'_, T> {
    self.as_mut_slice().iter_mut()
  }

  /// Extends the vector with the contents of an iterator. Items are
  /// pushed individually, potentially causing a spill from inline to
  /// heap if the total number of elements exceeds the inline capacity.
  pub fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
    for item in iter {
      self.push(item);
    }
  }

  /// Consumes the `CompactVec` and returns a standard `Vec<T>` with
  /// identical contents. This performs at most one allocation and
  /// moves all elements out of the inline storage if necessary.
  pub fn into_vec(mut self) -> Vec<T> {
    match self.heap.take() {
      Some(heap) => heap,
      None => {
        let mut vec = Vec::with_capacity(self.len);
        for i in 0..self.len {
          unsafe {
            vec.push(self.inline[i].assume_init_read());
          }
        }
        vec
      }
    }
  }
}

impl<T, const N: usize> Drop for CompactVec<T, N> {
  fn drop(&mut self) {
    match self.heap {
      Some(ref mut heap) => {
        // Dropping the Vec will drop its contents automatically.
        heap.clear();
      }
      None => {
        // Drop any initialized inline elements
        for i in 0..self.len {
          unsafe {
            self.inline[i].assume_init_drop();
          }
        }
      }
    }
  }
}

impl<T, const N: usize> Default for CompactVec<T, N> {
  fn default() -> Self {
    Self::new()
  }
}

impl<T, I: Into<usize>, const N: usize> Index<I> for CompactVec<T, N> {
  type Output = T;
  fn index(&self, index: I) -> &Self::Output {
    let index = index.into();
    match self.heap {
      Some(ref heap) => &heap[index],
      None => {
        assert!(index < self.len, "index out of bounds");
        // SAFETY: index < len ensures the element is initialized.
        unsafe { &*self.inline[index].as_ptr() }
      }
    }
  }
}

impl<T, I: Into<usize>, const N: usize> IndexMut<I> for CompactVec<T, N> {
  fn index_mut(&mut self, index: I) -> &mut Self::Output {
    let index = index.into();
    match self.heap {
      Some(ref mut heap) => &mut heap[index],
      None => {
        assert!(index < self.len, "index out of bounds");
        // SAFETY: index < len ensures the element is initialized.
        unsafe { &mut *self.inline[index].as_mut_ptr() }
      }
    }
  }
}

impl<T, const N: usize> Deref for CompactVec<T, N> {
  type Target = [T];
  fn deref(&self) -> &Self::Target {
    self.as_slice()
  }
}

impl<T, const N: usize> DerefMut for CompactVec<T, N> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.as_mut_slice()
  }
}

impl<T: fmt::Debug, const N: usize> fmt::Debug for CompactVec<T, N> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "CompactVec<{N}> {s:?}", s = self.as_slice())
  }
}

impl<T: Clone, const N: usize> Clone for CompactVec<T, N> {
  fn clone(&self) -> Self {
    if let Some(ref heap) = self.heap {
      Self {
        // SAFETY: uninitialized array is valid.
        inline: unsafe {
          MaybeUninit::<[MaybeUninit<T>; N]>::uninit().assume_init()
        },
        len:    0,
        heap:   Some(heap.clone()),
      }
    } else {
      let mut new_vec = Self::new();
      for item in self.as_slice() {
        new_vec.push(item.clone());
      }
      new_vec
    }
  }
}

impl<T: PartialEq, const N: usize> PartialEq for CompactVec<T, N> {
  fn eq(&self, other: &Self) -> bool {
    self.as_slice().eq(other.as_slice())
  }
}

impl<T: Eq, const N: usize> Eq for CompactVec<T, N> {}

impl<T: PartialOrd, const N: usize> PartialOrd for CompactVec<T, N> {
  fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
    self.as_slice().partial_cmp(other.as_slice())
  }
}

impl<T: Ord, const N: usize> Ord for CompactVec<T, N> {
  fn cmp(&self, other: &Self) -> core::cmp::Ordering {
    self.as_slice().cmp(other.as_slice())
  }
}

impl<T: core::hash::Hash, const N: usize> core::hash::Hash
  for CompactVec<T, N>
{
  fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
    self.as_slice().hash(state)
  }
}

impl<T, const N: usize> FromIterator<T> for CompactVec<T, N> {
  fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
    let mut vec = Self::new();
    vec.extend(iter);
    vec
  }
}

impl<T, const N: usize> IntoIterator for CompactVec<T, N> {
  type Item = T;
  type IntoIter = alloc::vec::IntoIter<T>;
  fn into_iter(self) -> Self::IntoIter {
    self.into_vec().into_iter()
  }
}

impl<'a, T, const N: usize> IntoIterator for &'a CompactVec<T, N> {
  type Item = &'a T;
  type IntoIter = core::slice::Iter<'a, T>;
  fn into_iter(self) -> Self::IntoIter {
    self.as_slice().iter()
  }
}

impl<'a, T, const N: usize> IntoIterator for &'a mut CompactVec<T, N> {
  type Item = &'a mut T;
  type IntoIter = core::slice::IterMut<'a, T>;
  fn into_iter(self) -> Self::IntoIter {
    self.as_mut_slice().iter_mut()
  }
}

#[cfg(feature = "serde")]
mod serde_impl {
  use super::*;

  impl<T, const N: usize> serde::Serialize for CompactVec<T, N>
  where
    T: serde::Serialize,
  {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
      S: serde::Serializer,
    {
      use serde::ser::SerializeSeq;
      let mut seq = serializer.serialize_seq(Some(self.len()))?;
      for elem in self.as_slice() {
        seq.serialize_element(elem)?;
      }
      seq.end()
    }
  }

  impl<'de, T, const N: usize> serde::Deserialize<'de> for CompactVec<T, N>
  where
    T: serde::Deserialize<'de>,
  {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
      D: serde::Deserializer<'de>,
    {
      use serde::de::SeqAccess;
      use serde::de::Visitor;
      struct CompactVecVisitor<T, const N: usize> {
        marker: core::marker::PhantomData<T>,
      }
      impl<'de, T, const N: usize> Visitor<'de> for CompactVecVisitor<T, N>
      where
        T: serde::Deserialize<'de>,
      {
        type Value = CompactVec<T, N>;
        fn expecting(
          &self,
          formatter: &mut core::fmt::Formatter,
        ) -> core::fmt::Result {
          formatter.write_str("a sequence")
        }
        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
          A: SeqAccess<'de>,
        {
          let mut vec = CompactVec::new();
          while let Some(value) = seq.next_element::<T>()? {
            vec.push(value);
          }
          Ok(vec)
        }
      }
      deserializer.deserialize_seq(CompactVecVisitor::<T, N> {
        marker: core::marker::PhantomData,
      })
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn inline_push_and_pop() {
    let mut vec: CompactVec<u32, 4> = CompactVec::new();
    assert!(vec.is_inline());
    assert_eq!(vec.len(), 0);
    // push within inline capacity
    vec.push(1);
    vec.push(2);
    vec.push(3);
    assert!(vec.is_inline());
    assert_eq!(vec.len(), 3);
    assert_eq!(vec.as_slice(), &[1, 2, 3]);
    // pop within inline capacity
    assert_eq!(vec.pop(), Some(3));
    assert_eq!(vec.len(), 2);
    assert!(vec.is_inline());
    assert_eq!(vec.as_slice(), &[1, 2]);
  }

  #[test]
  fn spill_and_recover() {
    let mut vec: CompactVec<u32, 3> = CompactVec::new();
    // push to spill onto the heap
    vec.push(10);
    vec.push(20);
    vec.push(30);
    vec.push(40);
    assert!(!vec.is_inline());
    assert_eq!(vec.as_slice(), &[10, 20, 30, 40]);
    // pop back down below inline capacity
    assert_eq!(vec.pop(), Some(40));
    assert_eq!(vec.len(), 3);
    assert!(vec.is_inline());
    assert_eq!(vec.as_slice(), &[10, 20, 30]);
  }

  #[test]
  fn extend_and_iterate() {
    let mut vec: CompactVec<u8, 2> = CompactVec::new();
    vec.extend([1u8, 2, 3, 4].iter().copied());
    assert_eq!(vec.len(), 4);
    assert_eq!(vec.as_slice(), &[1, 2, 3, 4]);
    // iterate over references
    let collected: Vec<u8> = vec.iter().copied().collect();
    assert_eq!(collected, &[1, 2, 3, 4]);
    // into_iter consumes and produces owned values
    let collected_owned: Vec<u8> = vec.clone().into_iter().collect();
    assert_eq!(collected_owned, &[1, 2, 3, 4]);
  }

  #[test]
  fn clone_and_eq() {
    let mut v1: CompactVec<i32, 2> = CompactVec::new();
    v1.extend([7, 8, 9].iter().cloned());
    let v2 = v1.clone();
    assert_eq!(v1, v2);
    // modify clone and ensure original is unaffected
    let mut v3 = v1.clone();
    v3.pop();
    assert_ne!(v1, v3);
    assert_eq!(v1.as_slice(), &[7, 8, 9]);
    assert_eq!(v3.as_slice(), &[7, 8]);
  }

  #[test]
  fn new_with_capacity_and_clear() {
    let vec: CompactVec<u8, 4> = CompactVec::new();
    assert!(vec.is_inline());
    assert!(vec.is_empty());
    assert_eq!(vec.capacity(), 4);

    let inline: CompactVec<u8, 4> = CompactVec::with_capacity(2);
    assert!(inline.is_inline());
    assert_eq!(inline.capacity(), 4);

    let heap: CompactVec<u8, 4> = CompactVec::with_capacity(12);
    assert!(!heap.is_inline());
    assert!(heap.capacity() >= 12);

    let mut vec: CompactVec<u8, 2> = CompactVec::new();
    vec.extend([1u8, 2]);
    vec.clear();
    assert!(vec.is_inline());
    assert!(vec.is_empty());

    let mut heap: CompactVec<u8, 1> = CompactVec::new();
    heap.extend([9u8, 10]);
    assert!(!heap.is_inline());
    heap.clear();
    assert!(heap.is_inline());
    assert!(heap.is_empty());
  }

  #[test]
  fn indexing_and_mutation() {
    let mut vec: CompactVec<i32, 2> = CompactVec::new();
    vec.extend([10, 20]);
    assert_eq!(vec[0usize], 10);
    vec[1usize] = 25;
    assert_eq!(vec.as_slice(), &[10, 25]);
  }

  #[test]
  #[should_panic(expected = "index out of bounds")]
  fn index_out_of_bounds_panics() {
    let vec: CompactVec<i32, 1> = CompactVec::new();
    let _ = vec[0usize];
  }

  #[test]
  fn iter_mut_and_as_mut_slice() {
    let mut vec: CompactVec<i32, 3> = CompactVec::new();
    vec.extend([1, 2, 3]);
    for v in vec.iter_mut() {
      *v *= 2;
    }
    assert_eq!(vec.as_slice(), &[2, 4, 6]);
    vec.as_mut_slice()[1] = 9;
    assert_eq!(vec.as_slice(), &[2, 9, 6]);
  }

  #[test]
  fn into_vec_and_into_iter() {
    let mut inline: CompactVec<u8, 2> = CompactVec::new();
    inline.extend([1u8, 2]);
    let vec = inline.into_vec();
    assert_eq!(vec, vec![1u8, 2]);

    let mut heap: CompactVec<u8, 1> = CompactVec::new();
    heap.extend([5u8, 6, 7]);
    let collected: Vec<u8> = heap.into_iter().collect();
    assert_eq!(collected, vec![5u8, 6, 7]);
  }

  #[test]
  fn from_iter_and_ref_iterators() {
    let vec: CompactVec<i32, 2> = [1, 2, 3].into_iter().collect();
    assert_eq!(vec.as_slice(), &[1, 2, 3]);
    let sum: i32 = (&vec).into_iter().copied().sum();
    assert_eq!(sum, 6);

    let mut vec2 = vec.clone();
    for v in &mut vec2 {
      *v += 1;
    }
    assert_eq!(vec2.as_slice(), &[2, 3, 4]);
  }

  #[test]
  fn ordering_and_hash() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut a: CompactVec<i32, 2> = CompactVec::new();
    a.extend([1, 2]);
    let mut b: CompactVec<i32, 2> = CompactVec::new();
    b.extend([1, 3]);
    assert!(a < b);

    let mut h1 = DefaultHasher::new();
    a.hash(&mut h1);
    let mut h2 = DefaultHasher::new();
    a.clone().hash(&mut h2);
    assert_eq!(h1.finish(), h2.finish());
  }

  #[test]
  fn zero_inline_capacity_spills_immediately() {
    let mut vec: CompactVec<i32, 0> = CompactVec::new();
    assert!(vec.is_inline());
    vec.push(1);
    assert!(!vec.is_inline());
    assert_eq!(vec.as_slice(), &[1]);
  }

  #[cfg(feature = "serde")]
  mod serde_tests {
    use super::*;
    use serde_json;

    #[test]
    fn serialize_and_deserialize() {
      let mut vec: CompactVec<u32, 2> = CompactVec::new();
      vec.push(42);
      vec.push(7);
      vec.push(99);
      let json = serde_json::to_string(&vec).unwrap();
      // Should serialize as a JSON array
      assert_eq!(json, "[42,7,99]");
      let de: CompactVec<u32, 2> = serde_json::from_str(&json).unwrap();
      assert_eq!(de.as_slice(), &[42, 7, 99]);
      assert_eq!(de.len(), 3);
    }
  }
}
