//! # MOOS
//!
//! ### Memory-Optimized Objects and Strings (_"moö-se"_)
//!
//! This crate (pronounced "moose") is a small collection of Rust primitives
//! that prioritize memory efficiency and performance in constrained/embedded
//! environments. At present, this crate includes 2 main types: [`CowStr`] and
//! [`InlineStr`], which are described in detail below.
//!
//! ---
//!
//! ## [`CowStr`]
//!
//! Memory-efficient string alternative to `Cow<'a, str>` from the
//! `std::borrow` module with memory optimizations and support for inline
//! storage of small strings on the stack via [`InlineStr`].
//!
//! ### Example
//!
//! ```rust
//! use moos::CowStr;
//!
//! # fn main() -> Result<(), moos::inline_str::StringTooLongError> {
//! let owned = CowStr::Owned("This is an owned string.".into());
//! let inlined = CowStr::Inlined("smol str!".parse()?);
//! let borrowed = CowStr::Borrowed("This is a borrowed string.");
//! # Ok(())
//! # }
//! ```
//!
//! ## [`InlineStr`]
//!
//! The [`InlineStr`] type is a low-level inline (stack-allocated) string type,
//! designed specifically for small strings. It avoids heap allocations for
//! strings within the size limit imposed by its fixed capacity, which is
//! dependent on the architecture's pointer width.
//!
//! ### Capacity
//!
//! The fixed capacity of an `InlineStr` is dependent on the pointer width of
//! the target architecture; it is designed to maximize the amount of inline
//! storage available within a single machine word, less 2 bytes for its length
//! and null terminator (`\0`) character.
//!
//! On 64-bit systems, this usually equates to a maximum size of 22 B of UTF-8
//! data, while on 32-bit systems, the maximum size is typically 10 B.
//!
//! ---
//!
//! ## `no_std` Support
//!
//! These types are designed to be used in `no_std` environments, making them
//! suitable for embedded systems and other resource-constrained applications.
//!
//! ---
//!
//! ## Features
//!
//! - `std`: Enables integration with the Rust standard library. When disabled,
//!   which is the default, the crate operates in `no_std` mode.
//! - `serde`†: Enables serialization and deserialization support via Serde.
//!
//! > † enabled by default

#![cfg_attr(not(any(test, feature = "std")), no_std)]

extern crate alloc;
extern crate core;

pub mod cow_str;
pub mod inline_str;

pub use cow_str::*;
pub use inline_str::*;
