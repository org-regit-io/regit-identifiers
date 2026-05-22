// Copyright 2026 Regit.io — Nicolas Koenig
// SPDX-License-Identifier: Apache-2.0

//! VALOR — Valorennummer (SIX Financial Information).
//!
//! A VALOR is the Swiss national securities-identifying number. It is a
//! purely numeric identifier of variable length, from 1 to 9 digits:
//!
//! ```text
//!   1 2 1 3 8 5 3
//!   └──────┬──────┘
//!          └ 1 to 9 digits [0-9]   no internal structure, no check digit
//! ```
//!
//! - The VALOR carries **no internal segments**: it is a single run of
//!   between 1 and 9 ASCII decimal digits.
//! - There is **no check digit** — validation is purely structural.
//! - A Swiss ISIN embeds the VALOR: `CH` + the VALOR left-padded with zeros
//!   to nine digits + the ISIN check digit. For example, VALOR `1213853`
//!   becomes ISIN `CH0012138530`.
//!
//! [`Valor::parse`] enforces every rule: a length of 1 to 9 characters and a
//! charset of ASCII digits only.
//!
//! # References
//!
//! - SIX Financial Information, *Valorennummer* — the Swiss national
//!   securities-identification scheme.

use crate::errors::ValidationError;

/// A validated Valorennummer (SIX Financial Information).
///
/// A `Valor` can only be created by [`Valor::parse`] (or the explicitly
/// unchecked [`Valor::from_bytes_unchecked`]), so a value of this type is a
/// proof that it holds between 1 and 9 ASCII decimal digits. It stores the
/// identifier inline as `[u8; 9]` plus a length, is `Copy`, and allocates
/// nothing.
///
/// Because the VALOR is variable-length, the unused tail bytes of the array
/// are always kept zeroed, so the derived `PartialEq`, `Eq`, and `Hash` are
/// correct: two `Valor` values compare equal exactly when their digits do.
///
/// # Examples
///
/// ```
/// use regit_identifiers::Valor;
///
/// let valor = Valor::parse("1213853").unwrap();
/// assert_eq!(valor.as_str(), "1213853");
/// assert_eq!(valor.len(), 7);
/// assert_eq!(valor.as_u64(), 1_213_853);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Valor {
    /// The validated ASCII digit bytes, left-aligned; unused tail is zeroed.
    bytes: [u8; Self::MAX_LENGTH],
    /// The number of significant bytes in `bytes`, always `1..=9`.
    len: u8,
}

impl Valor {
    /// The minimum number of digits in a VALOR.
    pub const MIN_LENGTH: usize = 1;

    /// The maximum number of digits in a VALOR.
    pub const MAX_LENGTH: usize = 9;

    /// Parses and fully validates a VALOR.
    ///
    /// Validation is strict and, in order: the input must be 1 to 9
    /// characters; every character must be an ASCII decimal digit. There is
    /// no check digit.
    ///
    /// # Errors
    ///
    /// - [`ValidationError::Structure`] with rule `"VALOR must be 1 to 9
    ///   digits"` if the input is empty or longer than nine characters.
    /// - [`ValidationError::InvalidCharacter`] if a character is not an ASCII
    ///   digit (this also rejects any non-ASCII character).
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Valor;
    /// use regit_identifiers::errors::ValidationError;
    ///
    /// assert!(Valor::parse("1213853").is_ok());
    ///
    /// // A non-digit character is rejected, not silently accepted.
    /// assert_eq!(
    ///     Valor::parse("1213853A"),
    ///     Err(ValidationError::InvalidCharacter { position: 8, found: 'A' }),
    /// );
    /// ```
    pub fn parse(s: &str) -> Result<Self, ValidationError> {
        // A VALOR is 1 to 9 characters; any other length is a structural
        // violation rather than a per-position length mismatch.
        let found = s.chars().count();
        if !(Self::MIN_LENGTH..=Self::MAX_LENGTH).contains(&found) {
            return Err(ValidationError::Structure {
                rule: "VALOR must be 1 to 9 digits",
            });
        }
        // Every character must be an ASCII digit. A non-ASCII character fails
        // the predicate and is rejected here.
        for (i, ch) in s.chars().enumerate() {
            if !ch.is_ascii_digit() {
                return Err(ValidationError::InvalidCharacter {
                    position: i + 1,
                    found: ch,
                });
            }
        }
        // Every character is ASCII, so the string is `found` ASCII bytes that
        // fit the array; the unused tail stays zeroed.
        let mut bytes = [0u8; Self::MAX_LENGTH];
        if let Some(slot) = bytes.get_mut(..found) {
            slot.copy_from_slice(s.as_bytes());
        }
        Ok(Self {
            bytes,
            len: u8::try_from(found).unwrap_or(0),
        })
    }

    /// Validates a VALOR without constructing one.
    ///
    /// Equivalent to `Valor::parse(s).map(|_| ())`; use it when only the
    /// verdict is needed.
    ///
    /// # Errors
    ///
    /// Returns the same [`ValidationError`] variants as [`Valor::parse`].
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Valor;
    ///
    /// assert!(Valor::validate("908440").is_ok());
    /// assert!(Valor::validate("1234567890").is_err());
    /// ```
    pub fn validate(s: &str) -> Result<(), ValidationError> {
        Self::parse(s).map(|_| ())
    }

    /// Wraps raw bytes as a `Valor` without any validation.
    ///
    /// The caller asserts that the first `len` bytes of `bytes` are ASCII
    /// decimal digits, that `len` is in `1..=9`, and that the remaining tail
    /// bytes are zero. This exists for reconstructing a `Valor` from bytes
    /// that were validated earlier; prefer [`Valor::parse`] for any untrusted
    /// input.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Valor;
    ///
    /// let valor = Valor::from_bytes_unchecked(*b"908440\0\0\0", 6);
    /// assert_eq!(valor.as_str(), "908440");
    /// ```
    #[must_use]
    pub const fn from_bytes_unchecked(bytes: [u8; Self::MAX_LENGTH], len: u8) -> Self {
        Self { bytes, len }
    }

    /// Returns the VALOR as a string slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Valor;
    ///
    /// assert_eq!(Valor::parse("24476758").unwrap().as_str(), "24476758");
    /// ```
    #[must_use]
    #[inline]
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(self.as_bytes()).unwrap_or("")
    }

    /// Returns the VALOR as its raw ASCII digit bytes.
    ///
    /// The slice has [`Valor::len`] elements; the zeroed tail of the backing
    /// array is not included.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Valor;
    ///
    /// assert_eq!(Valor::parse("908440").unwrap().as_bytes(), b"908440");
    /// ```
    #[must_use]
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.get(..self.len as usize).unwrap_or(&[])
    }

    /// Returns the number of digits in the VALOR, always in `1..=9`.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Valor;
    ///
    /// assert_eq!(Valor::parse("1213853").unwrap().len(), 7);
    /// ```
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// Always `false` — a valid VALOR is never empty (it has 1 to 9
    /// digits). Provided for API consistency with [`Valor::len`].
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Valor;
    ///
    /// assert!(!Valor::parse("1213853").unwrap().is_empty());
    /// ```
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the VALOR's numeric value.
    ///
    /// A VALOR is at most nine decimal digits, so its value always fits in a
    /// `u64` (and indeed in a `u32`); leading zeros are absorbed.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Valor;
    ///
    /// assert_eq!(Valor::parse("1213853").unwrap().as_u64(), 1_213_853);
    /// assert_eq!(Valor::parse("000123").unwrap().as_u64(), 123);
    /// ```
    #[must_use]
    #[inline]
    pub fn as_u64(&self) -> u64 {
        let mut value: u64 = 0;
        for &b in self.as_bytes() {
            value = value * 10 + u64::from(b.wrapping_sub(b'0'));
        }
        value
    }

    /// Returns the VALOR's numeric value as a `u32`.
    ///
    /// A nine-digit decimal fits comfortably in a `u32` (the maximum
    /// `999_999_999` is well under `2^32 ≈ 4.29 × 10^9`); this is the
    /// narrower companion of [`Valor::as_u64`].
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Valor;
    ///
    /// assert_eq!(Valor::parse("1213853").unwrap().as_u32(), 1_213_853);
    /// assert_eq!(Valor::parse("999999999").unwrap().as_u32(), 999_999_999);
    /// ```
    #[must_use]
    #[inline]
    pub fn as_u32(&self) -> u32 {
        // The maximum 9-digit decimal (999_999_999) is strictly below `2^32`,
        // so the conversion can never truncate.
        u32::try_from(self.as_u64()).unwrap_or(0)
    }
}

impl core::fmt::Display for Valor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl core::str::FromStr for Valor {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl AsRef<str> for Valor {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::display;
    use core::str::FromStr;

    /// Real, well-known VALORs used as regression anchors.
    const GOLDEN: &[&str] = &["1213853", "908440", "24476758"];

    #[test]
    fn parses_golden_valors() {
        for &s in GOLDEN {
            let valor = Valor::parse(s).unwrap_or_else(|e| panic!("{s} should parse: {e}"));
            assert_eq!(valor.as_str(), s);
        }
    }

    #[test]
    fn accepts_minimum_and_maximum_length() {
        let one = Valor::parse("7").unwrap();
        assert_eq!(one.len(), 1);
        assert_eq!(one.as_str(), "7");

        let nine = Valor::parse("123456789").unwrap();
        assert_eq!(nine.len(), 9);
        assert_eq!(nine.as_str(), "123456789");
        assert_eq!(Valor::MIN_LENGTH, 1);
        assert_eq!(Valor::MAX_LENGTH, 9);
    }

    #[test]
    fn accessors() {
        let valor = Valor::parse("24476758").unwrap();
        assert_eq!(valor.as_str(), "24476758");
        assert_eq!(valor.as_bytes(), b"24476758");
        assert_eq!(valor.len(), 8);
        assert_eq!(valor.as_u64(), 24_476_758);
    }

    #[test]
    fn as_u64_computes_numeric_value() {
        assert_eq!(Valor::parse("1213853").unwrap().as_u64(), 1_213_853);
        assert_eq!(Valor::parse("908440").unwrap().as_u64(), 908_440);
        assert_eq!(Valor::parse("0").unwrap().as_u64(), 0);
        // Leading zeros are absorbed into the numeric value.
        assert_eq!(Valor::parse("000123").unwrap().as_u64(), 123);
        // The largest possible VALOR still fits in a u64.
        assert_eq!(Valor::parse("999999999").unwrap().as_u64(), 999_999_999);
    }

    #[test]
    fn as_u32_agrees_with_as_u64() {
        for s in ["0", "1213853", "908440", "999999999", "000123"] {
            let v = Valor::parse(s).unwrap();
            assert_eq!(u64::from(v.as_u32()), v.as_u64(), "value {s}");
        }
        assert_eq!(Valor::parse("999999999").unwrap().as_u32(), 999_999_999);
    }

    #[test]
    fn rejects_empty_input() {
        assert_eq!(
            Valor::parse(""),
            Err(ValidationError::Structure {
                rule: "VALOR must be 1 to 9 digits",
            })
        );
    }

    #[test]
    fn rejects_too_long() {
        assert_eq!(
            Valor::parse("1234567890"),
            Err(ValidationError::Structure {
                rule: "VALOR must be 1 to 9 digits",
            })
        );
    }

    #[test]
    fn rejects_non_digit_character() {
        assert_eq!(
            Valor::parse("1213853A"),
            Err(ValidationError::InvalidCharacter {
                position: 8,
                found: 'A',
            })
        );
    }

    #[test]
    fn rejects_non_digit_at_first_position() {
        assert!(matches!(
            Valor::parse("X12345"),
            Err(ValidationError::InvalidCharacter { position: 1, .. })
        ));
    }

    #[test]
    fn rejects_non_ascii_without_panic() {
        // A multi-byte character must be rejected cleanly.
        assert!(Valor::parse("12345é").is_err());
        assert!(Valor::parse("é").is_err());
    }

    #[test]
    fn validate_matches_parse() {
        assert!(Valor::validate("908440").is_ok());
        assert!(Valor::validate("").is_err());
        assert!(Valor::validate("1234567890").is_err());
    }

    #[test]
    fn round_trips_through_str() {
        for &s in GOLDEN {
            assert_eq!(Valor::parse(s).unwrap().as_str(), s);
        }
    }

    #[test]
    fn from_str_matches_parse() {
        assert_eq!(Valor::from_str("1213853"), Valor::parse("1213853"));
        assert!(Valor::from_str("nonsense").is_err());
    }

    #[test]
    fn display_renders_identifier() {
        let valor = Valor::parse("1213853").unwrap();
        assert_eq!(display(valor).as_str(), "1213853");
    }

    #[test]
    fn as_ref_str() {
        let valor = Valor::parse("908440").unwrap();
        let s: &str = valor.as_ref();
        assert_eq!(s, "908440");
    }

    #[test]
    fn from_bytes_unchecked_round_trip() {
        let valor = Valor::from_bytes_unchecked(*b"908440\0\0\0", 6);
        assert_eq!(valor, Valor::parse("908440").unwrap());
    }

    #[test]
    fn unused_tail_is_zeroed_for_eq_and_hash() {
        // Two values of differing length must never compare equal, and a
        // value parsed twice must compare equal — the zeroed tail guarantees
        // the derived PartialEq/Eq/Hash are correct.
        let a = Valor::parse("12345").unwrap();
        let b = Valor::parse("12345").unwrap();
        assert_eq!(a, b);
        assert_ne!(a, Valor::parse("123456").unwrap());
        assert_ne!(a, Valor::parse("1234").unwrap());
    }

    #[test]
    fn is_copy_and_eq_and_hashable() {
        let a = Valor::parse("1213853").unwrap();
        let b = a; // Copy
        assert_eq!(a, b);
        assert_ne!(a, Valor::parse("908440").unwrap());
        // Usable as a map key (Eq + Hash) — checked by constructing a slice.
        let keys = [a, b];
        assert_eq!(keys[0], keys[1]);
    }
}
