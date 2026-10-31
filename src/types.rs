//! Fixed-width integer / float types.
//!
//! The upstream C port (`include/types.h`) relies on the N64 toolchain's
//! typedefs (`s8`, `u8`, `s16`, `u16`, `s32`, `u32`, `f32`, ...). On the PC
//! these resolve to the C compiler's widths, which for the values the game uses
//! are stable. We use the Rust primitives directly — they have the exact same
//! widths as the C types the game was written against, so the ported logic is
//! identical regardless of host compiler.
//!
//! The game code refers to these as `s8`/`u8`/`s16`/`f32`/etc. To keep the
//! ported code readable we re-export the primitives under those spellings from
//! this module. We do it via a `pub use` of the primitives, which does NOT
//! create a recursive alias (unlike `type u8 = u8`).

// Re-export the primitives under the game's preferred spellings.
#[allow(redundant_extern_keywords)]
pub use core::{
    f32, f64, i16, i32, i64, i8, u16, u32, u64, u8,
};

// The game code refers to signed ints as `s16`/`s32`/etc. These are plain
// aliases for the primitives (not recursive, since the RHS names are primitives).
pub type s16 = i16;
pub type s32 = i32;
pub type s64 = i64;
pub type s8 = i8;

/// `UNUSED` — marks a parameter/variable that is intentionally not referenced.
/// In C this expands to `__attribute__((unused))`; in Rust it's a no-op marker.
#[allow(dead_code)]
pub struct Unused;

/// Cast a value to `s16` (16-bit signed), matching C's wrap-around semantics.
#[inline]
pub fn s16_wrap(v: i32) -> i16 {
    (v as i16) as i16
}

/// Cast a value to `u16`.
#[inline]
pub fn u16_wrap(v: u32) -> u16 {
    (v as u16) as u16
}
