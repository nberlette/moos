<div align="center">

# [moos]

##### <u>M</u>emory-<u>O</u>ptimized <u>O</u>bjects and <u>S</u>trings <sup>_(`no_std`)_</sup>

</div>

---

## Overview

**`moos`** — pronounced _"moose"_ — is a small collection of memory-optimized
string types for Rust, implementing small string optimization (SSO) techniques
and copy-on-write (COW) semantics to minimize heap allocations and improve the
performance of string operations.

Designed for use in `no_std` environments, `moos` prioritizes performance,
memory efficiency, and interoperability with common Rust string types. It is
ideal for applications where memory usage is a concern, such as embedded systems
or real-time applications.

## Usage

```sh
cargo add moos
```

```toml
[dependencies]
  moos = "0.1"
```

---

## `CowStr<'a>`

Memory-optimized alternative to `Cow<'a, str>`. It supports a special
[`CowStr::Inlined`](#cowstrinlined) variant — in addition to
[`CowStr::Owned`](#cowstrowned) and [`CowStr::Borrowed`](#cowstrborrowed), like
its `std::borrow` counterpart — which allows
[small strings](#max_inline_str_len) to be stored inline on the stack, reducing
heap allocations and improving performance for small string operations common in
many applications.

### Variants

#### `CowStr::Owned`

Represents an owned string that is always heap-allocated.

```rust
use moos::CowStr;

let owned_str = CowStr::Owned(box "Owned string data.");

assert!(owned_str.is_owned());
```

#### `CowStr::Borrowed`

Represents a borrowed string slice (`&str`) that is stored on the stack.

```rust
use moos::CowStr;

let borrowed_str = CowStr::Borrowed("This is a borrowed &str.");

assert!(borrowed_str.is_borrowed());
```

#### `CowStr::Inlined`

Represents a small string that is stored inline on the stack. This variant is
used for strings that are shorter than or equal to [`MAX_INLINE_STR_LEN`].

```rust
use moos::CowStr;

let inlined_str = CowStr::Inlined("Inlined string".into());

assert!(inlined_str.is_inlined());
```

### Example

```rust
use moos::CowStr;

// `CowStr::Owned` variant - always heap-allocated
let owned_str = CowStr::from(String::from("Owned string data."));

// `CowStr::Inlined` variant - stored on the stack
let small_str = CowStr::from("Hello, world!"); // Stored inline on x64

// `CowStr::Borrowed` variant - stored on the stack
let large_str = CowStr::from("This string exceeds the inline limit.");

assert!(owned_str.is_owned());
assert!(small_str.is_inlined());
assert!(large_str.is_borrowed());
```

---

## `InlineStr`

The `InlineStr` type is a fixed-size string type that can store small strings
directly on the stack, up to a maximum length defined by [`MAX_INLINE_STR_LEN`].

This allows for efficient storage and manipulation of small strings without heap
allocations, making it ideal for performance-critical applications where memory
usage is a concern, such as embedded systems or real-time applications.

- [x] Supports UTF-8 encoded strings.
- [x] Provides conversion methods to and from standard string types.
- [x] Implements common traits like `Deref`, `AsRef<str>`, `Display`, `Debug`
- [x] Supports comparison and ordering operations.
- [x] Supports serialization/deserialization with **[serde]**
  > **Note**: Requires the `serde` feature flag to be enabled.

```rust
use moos::InlineStr;

// Create an InlineStr from a string slice
let inline_str = InlineStr::try_from("Hello, InlineStr!").unwrap();
// Implements the Display trait for easy printing
println!("InlineStr content: {inline_str}");

// Can be compared with regular strings and slices
assert_eq!(inline_str, "Hello, InlineStr!");

// Supports mutation of the underlying byte buffer
let mut mutable_inline_str = inline_str;
mutable_inline_str.as_bytes_mut()[7..14].copy_from_slice(b"World!!");
println!("Modified InlineStr content: {mutable_inline_str}");
```

Attempting to create an InlineStr from a string that is too long:

```rust
let long_string = "This string is longer than the max length for InlineStr.";
match InlineStr::try_from(long_string) {
  Ok(inline_str) => println!("Successfully created InlineStr: {inline_str}"),
  Err(e) => println!("Error creating InlineStr: {e}"),
}
```

### `MAX_INLINE_STR_LEN`

The constant `MAX_INLINE_STR_LEN` defines the maximum length of an inline string
in bytes, determined by the target architecture's pointer width. On 64-bit
systems, this is typically 22 B, while on 32-bit systems, it's usually 10 B.

> This value is calculated as 3 times the size of an `isize` (to account for
> UTF-8 encoding), minus 2 bytes to reserve space for a `u8` length byte and a
> null terminator (`\0`) character (not stored but conceptually present in a
> manner similar to C-style strings).

### `StringTooLongError`

The `StringTooLongError` is an error type returned when attempting to create an
`InlineStr` from a string or string slice (`&str`) that exceeds the maximum
allowed length defined by [`MAX_INLINE_STR_LEN`].

```rust
use moos::inline_str::{InlineStr, StringTooLongError};

// Attempt to create an InlineStr from a string that is too long
let long_string = "This string is longer than the max length for InlineStr.";

match InlineStr::try_from(long_string) {
  Ok(inline_str) => println!("Successfully created InlineStr: {inline_str}"),
  Err(e) => println!("Error creating InlineStr: {e}"),
}
```

---

<div align="center">

**[MIT] © [Nicholas Berlette].** All rights reserved.

<small>

[moos] · [github] · [issues] · [docs] · [contributing]

</small></div>

[MIT]: https://nick.mit-license.org/2025 "MIT © Nicholas Berlette. All rights reserved."
[Nicholas Berlette]: https://github.com/nberlette "Follow @nberlette on GitHub for more cool stuff!"
[`MAX_INLINE_STR_LEN`]: #max_inline_str_len
[serde]: https://crates.io/crates/serde "Serialization framework for Rust"
[moos]: https://crates.io/crates/moos "moos on crates.io"
[GitHub]: https://github.com/nberlette/moos "moos on GitHub"
[Issues]: https://github.com/nberlette/moos/issues "moos issues on GitHub"
[Docs]: https://docs.rs/moos "moos documentation on docs.rs"
[Contributing]: https://github.com/nberlette/moos/blob/main/.github/CONTRIBUTING.md "Contributing to moos on GitHub"
