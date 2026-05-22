// Copyright 2026 Regit.io — Nicolas Koenig
// SPDX-License-Identifier: Apache-2.0

//! Internal ASCII alphabet primitives.
//!
//! Securities identifiers are all-ASCII, uppercase, fixed-alphabet strings.
//! This module collects the small classification and value functions the
//! check-digit logic shares. Every function operates on a single `u8` byte;
//! none allocates, none panics, and all are `#[inline]`.
//!
//! The "value" functions ([`digit_value`], [`letter_ordinal`],
//! [`alnum_value`]) have a documented precondition that the caller has
//! already established with the matching classifier. They never panic — an
//! out-of-range byte yields a defined but meaningless number — but the
//! contract is that they are only called on bytes the classifier accepted.
//!
//! This module is `pub(crate)`: it carries no public API.
//!
//! # References
//!
//! - ANSI X3.4 (ASCII) — the character encoding all identifiers use.

// ─── Classifiers ─────────────────────────────────────────────────────────────

/// Returns `true` if `b` is an ASCII decimal digit `'0'..='9'`.
#[inline]
pub(crate) fn is_digit(b: u8) -> bool {
    b.is_ascii_digit()
}

/// Returns `true` if `b` is an uppercase ASCII vowel (`A`, `E`, `I`, `O`,
/// `U`).
///
/// SEDOL and FIGI bodies forbid vowels so a check character can never be
/// mistaken for part of a word; identifier parsers use this to reject them.
#[inline]
pub(crate) fn is_vowel(b: u8) -> bool {
    matches!(b, b'A' | b'E' | b'I' | b'O' | b'U')
}

// ─── Value functions ─────────────────────────────────────────────────────────

/// Maps an ASCII digit byte to its numeric value, e.g. `b'7'` to `7`.
///
/// The caller must have established `is_digit(b)`. A non-digit byte yields a
/// defined but meaningless value rather than panicking.
#[inline]
pub(crate) fn digit_value(b: u8) -> u32 {
    u32::from(b.wrapping_sub(b'0'))
}

/// Maps an uppercase ASCII letter to its 1-based ordinal: `b'A'` to `1`,
/// `b'Z'` to `26`.
///
/// The caller must have established that `b` is an upper-case ASCII letter.
/// A non-letter byte yields a defined but meaningless value rather than
/// panicking.
#[inline]
pub(crate) fn letter_ordinal(b: u8) -> u32 {
    u32::from(b.wrapping_sub(b'A')).wrapping_add(1)
}

/// Maps an alphanumeric byte to its base-36 value: `'0'..='9'` to `0..=9`,
/// `'A'..='Z'` to `10..=35`.
///
/// This is the expansion ISIN, LEI, CUSIP, SEDOL, and FIGI all use to fold
/// letters into their check-digit arithmetic (`A = 10 + ('A' - 'A')`). The
/// caller must have established that `b` is an ASCII digit or an upper-case
/// letter; a byte outside that set yields a defined but meaningless value
/// rather than panicking.
#[inline]
pub(crate) fn alnum_value(b: u8) -> u32 {
    if is_digit(b) {
        digit_value(b)
    } else {
        letter_ordinal(b).wrapping_add(9)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_digit_classifies() {
        assert!(is_digit(b'0'));
        assert!(is_digit(b'9'));
        assert!(!is_digit(b'A'));
        assert!(!is_digit(b'/'));
        assert!(!is_digit(b' '));
    }

    #[test]
    fn is_vowel_classifies() {
        for v in [b'A', b'E', b'I', b'O', b'U'] {
            assert!(is_vowel(v));
        }
        for c in [b'B', b'C', b'Y', b'Z', b'0'] {
            assert!(!is_vowel(c));
        }
    }

    #[test]
    fn digit_value_maps() {
        assert_eq!(digit_value(b'0'), 0);
        assert_eq!(digit_value(b'7'), 7);
        assert_eq!(digit_value(b'9'), 9);
    }

    #[test]
    fn letter_ordinal_maps() {
        assert_eq!(letter_ordinal(b'A'), 1);
        assert_eq!(letter_ordinal(b'B'), 2);
        assert_eq!(letter_ordinal(b'Z'), 26);
    }

    #[test]
    fn alnum_value_maps() {
        assert_eq!(alnum_value(b'0'), 0);
        assert_eq!(alnum_value(b'9'), 9);
        assert_eq!(alnum_value(b'A'), 10);
        assert_eq!(alnum_value(b'S'), 28);
        assert_eq!(alnum_value(b'U'), 30);
        assert_eq!(alnum_value(b'Z'), 35);
    }

    #[test]
    fn value_functions_never_panic_on_any_byte() {
        for b in 0u8..=255 {
            let _ = digit_value(b);
            let _ = letter_ordinal(b);
            let _ = alnum_value(b);
        }
    }
}
