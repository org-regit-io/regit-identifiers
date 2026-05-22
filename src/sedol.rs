// Copyright 2026 Regit.io — Nicolas Koenig
// SPDX-License-Identifier: Apache-2.0

//! SEDOL — Stock Exchange Daily Official List number (London Stock Exchange).
//!
//! A SEDOL is the national securities identifier for instruments listed in
//! the United Kingdom and Ireland. It is exactly 7 characters in two
//! segments:
//!
//! ```text
//!   0 2 6 3 4 9 4
//!   └────┬────┘ │
//!        │       └ check digit  [6]      one digit [0-9]
//!        └──────── body          [0..6]   six characters [0-9 + consonants]
//! ```
//!
//! - The **body** is six characters drawn from the digits and the consonants
//!   — a vowel (`A`, `E`, `I`, `O`, `U`) is never used, so a check character
//!   can never be confused for part of a word. Legacy SEDOLs, issued before
//!   the 2004 switch to an alphanumeric scheme, are purely numeric.
//! - The **check digit** is the weighted-sum modulus of the six-character
//!   body — see [`crate::checkdigit::sedol_check_digit`].
//!
//! [`Sedol::parse`] enforces every rule: exact length, the body character
//! set with vowels rejected, a digit in the check position, and a check
//! digit that is recomputed and verified — never trusted.
//!
//! # References
//!
//! - London Stock Exchange — SEDOL Masterfile service description.

use crate::checkdigit;
use crate::errors::ValidationError;

/// A validated Stock Exchange Daily Official List number (SEDOL).
///
/// A `Sedol` can only be created by [`Sedol::parse`] (or the explicitly
/// unchecked [`Sedol::from_bytes_unchecked`]), so a value of this type is a
/// proof that the 7 characters form a structurally valid SEDOL with a
/// correct check digit. It stores the identifier inline as `[u8; 7]`, is
/// `Copy`, and allocates nothing.
///
/// # Examples
///
/// ```
/// use regit_identifiers::Sedol;
///
/// let sedol = Sedol::parse("0263494").unwrap();
/// assert_eq!(sedol.body(), "026349");
/// assert_eq!(sedol.check_digit(), '4');
/// assert_eq!(sedol.as_str(), "0263494");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sedol {
    /// The 7 validated ASCII bytes of the identifier.
    bytes: [u8; Self::LENGTH],
}

impl Sedol {
    /// The number of characters in a SEDOL.
    pub const LENGTH: usize = 7;

    /// Parses and fully validates a SEDOL.
    ///
    /// Validation is strict and, in order: the input must be exactly 7
    /// characters; characters 1–6 must each be an ASCII digit or an
    /// upper-case consonant (a vowel is rejected); character 7 must be an
    /// ASCII digit; and the check digit must equal the value recomputed from
    /// the six-character body.
    ///
    /// # Errors
    ///
    /// - [`ValidationError::WrongLength`] if the input is not 7 characters.
    /// - [`ValidationError::InvalidCharacter`] if a character falls outside
    ///   the set its position allows — this rejects a vowel in the body, a
    ///   non-digit check character, lower-case input, and any non-ASCII
    ///   character.
    /// - [`ValidationError::BadCheckDigit`] if the supplied check digit does
    ///   not match the recomputed one.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Sedol;
    /// use regit_identifiers::errors::ValidationError;
    ///
    /// assert!(Sedol::parse("0263494").is_ok());
    ///
    /// // A single wrong digit is caught, not silently accepted.
    /// assert_eq!(
    ///     Sedol::parse("0263495"),
    ///     Err(ValidationError::BadCheckDigit { expected: '4', found: '5' }),
    /// );
    /// ```
    pub fn parse(s: &str) -> Result<Self, ValidationError> {
        // A SEDOL is exactly 7 characters.
        let found = s.chars().count();
        if found != Self::LENGTH {
            return Err(ValidationError::WrongLength {
                expected: Self::LENGTH,
                found,
            });
        }
        // Per-position character set: [0..6] are digits or non-vowel A-Z,
        // [6] is a digit. A non-ASCII character fails every predicate and is
        // rejected here.
        for (i, ch) in s.chars().enumerate() {
            let legal = if i == Self::LENGTH - 1 {
                ch.is_ascii_digit()
            } else {
                ch.is_ascii_digit()
                    || (ch.is_ascii_uppercase() && !crate::charset::is_vowel(ch as u8))
            };
            if !legal {
                return Err(ValidationError::InvalidCharacter {
                    position: i + 1,
                    found: ch,
                });
            }
        }
        // Every character is ASCII, so the string is exactly 7 ASCII bytes.
        let mut bytes = [0u8; Self::LENGTH];
        bytes.copy_from_slice(s.as_bytes());

        // Recompute the check digit from the six-character body and compare.
        let body = core::str::from_utf8(&bytes[0..6]).unwrap_or("");
        let expected = checkdigit::sedol_check_digit(body)?;
        let supplied = char::from(bytes[6]);
        if expected != supplied {
            return Err(ValidationError::BadCheckDigit {
                expected,
                found: supplied,
            });
        }
        Ok(Self { bytes })
    }

    /// Validates a SEDOL without constructing one.
    ///
    /// Equivalent to `Sedol::parse(s).map(|_| ())`; use it when only the
    /// verdict is needed.
    ///
    /// # Errors
    ///
    /// Returns the same [`ValidationError`] variants as [`Sedol::parse`].
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Sedol;
    ///
    /// assert!(Sedol::validate("0263494").is_ok());
    /// assert!(Sedol::validate("0263495").is_err());
    /// ```
    pub fn validate(s: &str) -> Result<(), ValidationError> {
        Self::parse(s).map(|_| ())
    }

    /// Wraps 7 raw bytes as a `Sedol` without any validation.
    ///
    /// The caller asserts that `bytes` holds the 7 ASCII characters of a
    /// valid SEDOL. This exists for reconstructing a `Sedol` from bytes that
    /// were validated earlier; prefer [`Sedol::parse`] for any untrusted
    /// input.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Sedol;
    ///
    /// let sedol = Sedol::from_bytes_unchecked(*b"0263494");
    /// assert_eq!(sedol.as_str(), "0263494");
    /// ```
    #[must_use]
    pub const fn from_bytes_unchecked(bytes: [u8; Self::LENGTH]) -> Self {
        Self { bytes }
    }

    /// Returns the SEDOL as a string slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Sedol;
    ///
    /// assert_eq!(Sedol::parse("0263494").unwrap().as_str(), "0263494");
    /// ```
    #[must_use]
    #[inline]
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes).unwrap_or("")
    }

    /// Returns the SEDOL as its 7 raw ASCII bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Sedol;
    ///
    /// assert_eq!(Sedol::parse("0263494").unwrap().as_bytes(), b"0263494");
    /// ```
    #[must_use]
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the six-character body, characters 1–6.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Sedol;
    ///
    /// assert_eq!(Sedol::parse("0263494").unwrap().body(), "026349");
    /// ```
    #[must_use]
    #[inline]
    pub fn body(&self) -> &str {
        core::str::from_utf8(&self.bytes[0..6]).unwrap_or("")
    }

    /// Returns the check digit, character 7.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Sedol;
    ///
    /// assert_eq!(Sedol::parse("0263494").unwrap().check_digit(), '4');
    /// ```
    #[must_use]
    #[inline]
    pub fn check_digit(&self) -> char {
        char::from(self.bytes[6])
    }

    /// Returns `true` if this is a legacy purely-numeric SEDOL.
    ///
    /// SEDOLs issued before the 2004 switch to an alphanumeric scheme have a
    /// body consisting solely of digits; this is `true` exactly when all six
    /// body characters are ASCII digits.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Sedol;
    ///
    /// assert!(Sedol::parse("0263494").unwrap().is_legacy_numeric());
    /// assert!(!Sedol::parse("B0WNLY7").unwrap().is_legacy_numeric());
    /// ```
    #[must_use]
    #[inline]
    pub fn is_legacy_numeric(&self) -> bool {
        self.bytes[0..6].iter().all(u8::is_ascii_digit)
    }
}

impl core::fmt::Display for Sedol {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl core::str::FromStr for Sedol {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl AsRef<str> for Sedol {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::display;
    use core::str::FromStr;

    /// Real, well-known SEDOLs used as regression anchors.
    const GOLDEN: &[&str] = &[
        "0263494", // BAE Systems plc
        "0540528", // a second legacy numeric SEDOL
        "B0WNLY7", // a post-2004 alphanumeric SEDOL
    ];

    #[test]
    fn parses_golden_sedols() {
        for &s in GOLDEN {
            let sedol = Sedol::parse(s).unwrap_or_else(|e| panic!("{s} should parse: {e}"));
            assert_eq!(sedol.as_str(), s);
        }
    }

    #[test]
    fn segment_accessors() {
        let sedol = Sedol::parse("0263494").unwrap();
        assert_eq!(sedol.body(), "026349");
        assert_eq!(sedol.check_digit(), '4');
        assert_eq!(sedol.as_bytes(), b"0263494");
        assert_eq!(Sedol::LENGTH, 7);
    }

    #[test]
    fn accepts_alphanumeric_body() {
        // A post-2004 SEDOL with consonants in the body.
        let sedol = Sedol::parse("B0WNLY7").unwrap();
        assert_eq!(sedol.body(), "B0WNLY");
        assert_eq!(sedol.check_digit(), '7');
    }

    #[test]
    fn is_legacy_numeric_classifies() {
        assert!(Sedol::parse("0263494").unwrap().is_legacy_numeric());
        assert!(Sedol::parse("0540528").unwrap().is_legacy_numeric());
        assert!(!Sedol::parse("B0WNLY7").unwrap().is_legacy_numeric());
    }

    #[test]
    fn rejects_bad_check_digit() {
        assert_eq!(
            Sedol::parse("0263495"),
            Err(ValidationError::BadCheckDigit {
                expected: '4',
                found: '5',
            })
        );
    }

    #[test]
    fn rejects_wrong_length() {
        assert_eq!(
            Sedol::parse("026349"),
            Err(ValidationError::WrongLength {
                expected: 7,
                found: 6,
            })
        );
        assert_eq!(
            Sedol::parse(""),
            Err(ValidationError::WrongLength {
                expected: 7,
                found: 0,
            })
        );
    }

    #[test]
    fn rejects_vowel_in_body() {
        // A vowel can never appear in a SEDOL body.
        assert_eq!(
            Sedol::parse("B0WNLA7"),
            Err(ValidationError::InvalidCharacter {
                position: 6,
                found: 'A',
            })
        );
    }

    #[test]
    fn rejects_lower_case() {
        assert!(matches!(
            Sedol::parse("b0wnly7"),
            Err(ValidationError::InvalidCharacter { position: 1, .. })
        ));
    }

    #[test]
    fn rejects_non_digit_check_position() {
        // Character 7 must be a digit.
        assert!(matches!(
            Sedol::parse("026349B"),
            Err(ValidationError::InvalidCharacter { position: 7, .. })
        ));
    }

    #[test]
    fn rejects_non_ascii_without_panic() {
        // A multi-byte character must be rejected cleanly.
        assert!(Sedol::parse("026349é").is_err());
        assert!(Sedol::parse("é263494").is_err());
    }

    #[test]
    fn round_trips_through_str() {
        for &s in GOLDEN {
            assert_eq!(Sedol::parse(s).unwrap().as_str(), s);
        }
    }

    #[test]
    fn from_str_matches_parse() {
        assert_eq!(Sedol::from_str("0263494"), Sedol::parse("0263494"));
        assert!(Sedol::from_str("nonsense").is_err());
    }

    #[test]
    fn display_renders_identifier() {
        let sedol = Sedol::parse("0263494").unwrap();
        assert_eq!(display(sedol).as_str(), "0263494");
    }

    #[test]
    fn as_ref_str() {
        let sedol = Sedol::parse("0263494").unwrap();
        let s: &str = sedol.as_ref();
        assert_eq!(s, "0263494");
    }

    #[test]
    fn from_bytes_unchecked_round_trip() {
        let sedol = Sedol::from_bytes_unchecked(*b"0263494");
        assert_eq!(sedol, Sedol::parse("0263494").unwrap());
    }

    #[test]
    fn is_copy_and_eq_and_hashable() {
        let a = Sedol::parse("0263494").unwrap();
        let b = a; // Copy
        assert_eq!(a, b);
        assert_ne!(a, Sedol::parse("B0WNLY7").unwrap());
        // Usable as a map key (Eq + Hash) — checked by constructing a slice.
        let keys = [a, b];
        assert_eq!(keys[0], keys[1]);
    }
}
