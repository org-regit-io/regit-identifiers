// Copyright 2026 Regit.io — Nicolas Koenig
// SPDX-License-Identifier: Apache-2.0

//! LEI — Legal Entity Identifier (ISO 17442).
//!
//! An LEI is the globally unique reference code for a legal entity that
//! participates in a financial transaction. It is exactly 20 characters in
//! four segments:
//!
//! ```text
//!   5 4 9 3 0 0 1 K J T I I G C 8 Y 1 R 1 2
//!   └──┬──┘ └┬┘ └──────┬──────┘ └┬┘
//!      │     │         │         └ check digits [18..20]  two digits [0-9]
//!      │     │         └─────────── entity ID   [6..18]   twelve chars [A-Z0-9]
//!      │     └───────────────────── reserved    [4..6]    the literal "00"
//!      └─────────────────────────── LOU prefix  [0..4]    four chars [A-Z0-9]
//! ```
//!
//! - The **LOU prefix** identifies the Local Operating Unit that issued the
//!   identifier; it carries no further structure here.
//! - Positions 5–6 are a **reserved** field, fixed by the standard to the
//!   literal `00`.
//! - The **entity ID** is the LOU-assigned unique reference for the entity.
//!   (ISO 17442 calls this segment the *entity-specific part*; this crate's
//!   accessor is [`Lei::entity_id`].)
//! - The **check digits** are the ISO 7064 MOD 97-10 of the 18-character body
//!   — see [`crate::checkdigit::lei_check_digits`].
//!
//! [`Lei::parse`] enforces every rule: exact length, the per-segment
//! character set, the reserved `00` field, and check digits that are
//! recomputed and verified — never trusted.
//!
//! # References
//!
//! - ISO 17442, *Financial services — Legal entity identifier (LEI)*.
//! - ISO/IEC 7064, *Information technology — Security techniques — Check
//!   character systems* (the MOD 97-10 system).

use crate::checkdigit;
use crate::errors::ValidationError;

/// A validated Legal Entity Identifier (ISO 17442).
///
/// A `Lei` can only be created by [`Lei::parse`] (or the explicitly unchecked
/// [`Lei::from_bytes_unchecked`]), so a value of this type is a proof that the
/// 20 characters form a structurally valid LEI with correct check digits. It
/// stores the identifier inline as `[u8; 20]`, is `Copy`, and allocates
/// nothing.
///
/// # Examples
///
/// ```
/// use regit_identifiers::Lei;
///
/// let lei = Lei::parse("5493001KJTIIGC8Y1R12").unwrap();
/// assert_eq!(lei.lou_prefix(), "5493");
/// assert_eq!(lei.entity_id(), "1KJTIIGC8Y1R");
/// assert_eq!(lei.check_digits(), "12");
/// assert_eq!(lei.as_str(), "5493001KJTIIGC8Y1R12");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Lei {
    /// The 20 validated ASCII bytes of the identifier.
    bytes: [u8; Self::LENGTH],
}

impl Lei {
    /// The number of characters in an LEI.
    pub const LENGTH: usize = 20;

    /// Parses and fully validates an LEI.
    ///
    /// Validation is strict and, in order: the input must be exactly 20
    /// characters; characters 1–18 must each be an ASCII digit or upper-case
    /// letter and characters 19–20 ASCII digits; characters 5–6 must be the
    /// reserved literal `00`; and the two check digits must equal the values
    /// recomputed from the 18-character body.
    ///
    /// # Errors
    ///
    /// - [`ValidationError::WrongLength`] if the input is not 20 characters.
    /// - [`ValidationError::InvalidCharacter`] if a character falls outside
    ///   the set its position allows (this also rejects lower-case input and
    ///   any non-ASCII character).
    /// - [`ValidationError::Structure`] if characters 5–6 are not `00`.
    /// - [`ValidationError::BadCheckDigit`] if a supplied check digit does not
    ///   match the recomputed one; the first differing digit is reported.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Lei;
    /// use regit_identifiers::errors::ValidationError;
    ///
    /// assert!(Lei::parse("5493001KJTIIGC8Y1R12").is_ok());
    ///
    /// // A single wrong check digit is caught, not silently accepted.
    /// assert_eq!(
    ///     Lei::parse("5493001KJTIIGC8Y1R13"),
    ///     Err(ValidationError::BadCheckDigit { expected: '2', found: '3' }),
    /// );
    /// ```
    pub fn parse(s: &str) -> Result<Self, ValidationError> {
        // An LEI is exactly 20 characters.
        let found = s.chars().count();
        if found != Self::LENGTH {
            return Err(ValidationError::WrongLength {
                expected: Self::LENGTH,
                found,
            });
        }
        // Per-position character set: [0..18] are [A-Z0-9], [18..20] are
        // digits. A non-ASCII character fails both predicates and is rejected
        // here.
        for (i, ch) in s.chars().enumerate() {
            let legal = if i >= Self::LENGTH - 2 {
                ch.is_ascii_digit()
            } else {
                ch.is_ascii_digit() || ch.is_ascii_uppercase()
            };
            if !legal {
                return Err(ValidationError::InvalidCharacter {
                    position: i + 1,
                    found: ch,
                });
            }
        }
        // Every character is ASCII, so the string is exactly 20 ASCII bytes.
        let mut bytes = [0u8; Self::LENGTH];
        bytes.copy_from_slice(s.as_bytes());

        // Characters 5–6 are a reserved field fixed to the literal "00".
        let reserved = core::str::from_utf8(&bytes[4..6]).unwrap_or("");
        if reserved != "00" {
            return Err(ValidationError::Structure {
                rule: "LEI positions 5-6 must be 00",
            });
        }
        // Recompute the check digits from the 18-character body and compare,
        // reporting the first position where they differ.
        let body = core::str::from_utf8(&bytes[0..18]).unwrap_or("");
        let expected = checkdigit::lei_check_digits(body)?;
        for offset in 0..2 {
            let want = expected[offset];
            let got = char::from(bytes[18 + offset]);
            if want != got {
                return Err(ValidationError::BadCheckDigit {
                    expected: want,
                    found: got,
                });
            }
        }
        Ok(Self { bytes })
    }

    /// Validates an LEI without constructing one.
    ///
    /// Equivalent to `Lei::parse(s).map(|_| ())`; use it when only the verdict
    /// is needed.
    ///
    /// # Errors
    ///
    /// Returns the same [`ValidationError`] variants as [`Lei::parse`].
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Lei;
    ///
    /// assert!(Lei::validate("5493001KJTIIGC8Y1R12").is_ok());
    /// assert!(Lei::validate("5493001KJTIIGC8Y1R13").is_err());
    /// ```
    pub fn validate(s: &str) -> Result<(), ValidationError> {
        Self::parse(s).map(|_| ())
    }

    /// Wraps 20 raw bytes as a `Lei` without any validation.
    ///
    /// The caller asserts that `bytes` holds the 20 ASCII characters of a
    /// valid LEI. This exists for reconstructing a `Lei` from bytes that were
    /// validated earlier; prefer [`Lei::parse`] for any untrusted input.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Lei;
    ///
    /// let lei = Lei::from_bytes_unchecked(*b"5493001KJTIIGC8Y1R12");
    /// assert_eq!(lei.as_str(), "5493001KJTIIGC8Y1R12");
    /// ```
    #[must_use]
    pub const fn from_bytes_unchecked(bytes: [u8; Self::LENGTH]) -> Self {
        Self { bytes }
    }

    /// Returns the LEI as a string slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Lei;
    ///
    /// assert_eq!(
    ///     Lei::parse("5493001KJTIIGC8Y1R12").unwrap().as_str(),
    ///     "5493001KJTIIGC8Y1R12",
    /// );
    /// ```
    #[must_use]
    #[inline]
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes).unwrap_or("")
    }

    /// Returns the LEI as its 20 raw ASCII bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Lei;
    ///
    /// assert_eq!(
    ///     Lei::parse("5493001KJTIIGC8Y1R12").unwrap().as_bytes(),
    ///     b"5493001KJTIIGC8Y1R12",
    /// );
    /// ```
    #[must_use]
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the four-character LOU prefix, characters 1–4.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Lei;
    ///
    /// assert_eq!(
    ///     Lei::parse("5493001KJTIIGC8Y1R12").unwrap().lou_prefix(),
    ///     "5493",
    /// );
    /// ```
    #[must_use]
    #[inline]
    pub fn lou_prefix(&self) -> &str {
        core::str::from_utf8(&self.bytes[0..4]).unwrap_or("")
    }

    /// Returns the twelve-character entity ID, characters 7–18 (the segment
    /// ISO 17442 calls the *entity-specific part*).
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Lei;
    ///
    /// assert_eq!(
    ///     Lei::parse("5493001KJTIIGC8Y1R12").unwrap().entity_id(),
    ///     "1KJTIIGC8Y1R",
    /// );
    /// ```
    #[must_use]
    #[inline]
    pub fn entity_id(&self) -> &str {
        core::str::from_utf8(&self.bytes[6..18]).unwrap_or("")
    }

    /// Returns the two check digits, characters 19–20.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Lei;
    ///
    /// assert_eq!(
    ///     Lei::parse("5493001KJTIIGC8Y1R12").unwrap().check_digits(),
    ///     "12",
    /// );
    /// ```
    #[must_use]
    #[inline]
    pub fn check_digits(&self) -> &str {
        core::str::from_utf8(&self.bytes[18..20]).unwrap_or("")
    }
}

impl core::fmt::Display for Lei {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl core::str::FromStr for Lei {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl AsRef<str> for Lei {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::display;
    use core::str::FromStr;

    /// Real, well-known LEIs used as regression anchors.
    const GOLDEN: &[&str] = &[
        "5493001KJTIIGC8Y1R12", // Bloomberg Finance L.P.
        "549300DTUYXVMJXZNY75", // a second real LEI
    ];

    #[test]
    fn parses_golden_leis() {
        for &s in GOLDEN {
            let lei = Lei::parse(s).unwrap_or_else(|e| panic!("{s} should parse: {e}"));
            assert_eq!(lei.as_str(), s);
        }
    }

    #[test]
    fn segment_accessors() {
        let lei = Lei::parse("5493001KJTIIGC8Y1R12").unwrap();
        assert_eq!(lei.lou_prefix(), "5493");
        assert_eq!(lei.entity_id(), "1KJTIIGC8Y1R");
        assert_eq!(lei.check_digits(), "12");
        assert_eq!(lei.as_bytes(), b"5493001KJTIIGC8Y1R12");
        assert_eq!(Lei::LENGTH, 20);
    }

    #[test]
    fn rejects_bad_check_digit() {
        // The first differing digit is reported.
        assert_eq!(
            Lei::parse("5493001KJTIIGC8Y1R13"),
            Err(ValidationError::BadCheckDigit {
                expected: '2',
                found: '3',
            })
        );
    }

    #[test]
    fn rejects_bad_first_check_digit() {
        // When both digits are wrong, the first one is reported.
        assert_eq!(
            Lei::parse("5493001KJTIIGC8Y1R99"),
            Err(ValidationError::BadCheckDigit {
                expected: '1',
                found: '9',
            })
        );
    }

    #[test]
    fn rejects_wrong_length() {
        assert_eq!(
            Lei::parse("5493001KJTIIGC8Y1R1"),
            Err(ValidationError::WrongLength {
                expected: 20,
                found: 19,
            })
        );
        assert_eq!(
            Lei::parse("5493001KJTIIGC8Y1R123"),
            Err(ValidationError::WrongLength {
                expected: 20,
                found: 21,
            })
        );
        assert_eq!(
            Lei::parse(""),
            Err(ValidationError::WrongLength {
                expected: 20,
                found: 0,
            })
        );
    }

    #[test]
    fn rejects_lower_case() {
        assert!(matches!(
            Lei::parse("5493001kJTIIGC8Y1R12"),
            Err(ValidationError::InvalidCharacter { position: 8, .. })
        ));
    }

    #[test]
    fn rejects_non_digit_check_position() {
        // Characters 19–20 must be digits.
        assert!(matches!(
            Lei::parse("5493001KJTIIGC8Y1RX2"),
            Err(ValidationError::InvalidCharacter { position: 19, .. })
        ));
        assert!(matches!(
            Lei::parse("5493001KJTIIGC8Y1R1X"),
            Err(ValidationError::InvalidCharacter { position: 20, .. })
        ));
    }

    #[test]
    fn rejects_bad_body_character() {
        assert_eq!(
            Lei::parse("5493001KJTIIGC8Y1-12"),
            Err(ValidationError::InvalidCharacter {
                position: 18,
                found: '-',
            })
        );
    }

    #[test]
    fn rejects_reserved_field_not_zero_zero() {
        // Positions 5–6 must be the literal "00".
        assert_eq!(
            Lei::parse("5493011KJTIIGC8Y1R12"),
            Err(ValidationError::Structure {
                rule: "LEI positions 5-6 must be 00",
            })
        );
        assert_eq!(
            Lei::parse("5493A01KJTIIGC8Y1R12"),
            Err(ValidationError::Structure {
                rule: "LEI positions 5-6 must be 00",
            })
        );
    }

    #[test]
    fn rejects_non_ascii_without_panic() {
        // A multi-byte character must be rejected cleanly.
        assert!(Lei::parse("5493001KJTIIGC8Y1Ré2").is_err());
        assert!(Lei::parse("É493001KJTIIGC8Y1R12").is_err());
    }

    #[test]
    fn round_trips_through_str() {
        for &s in GOLDEN {
            assert_eq!(Lei::parse(s).unwrap().as_str(), s);
        }
    }

    #[test]
    fn from_str_matches_parse() {
        assert_eq!(
            Lei::from_str("5493001KJTIIGC8Y1R12"),
            Lei::parse("5493001KJTIIGC8Y1R12")
        );
        assert!(Lei::from_str("nonsense").is_err());
    }

    #[test]
    fn display_renders_identifier() {
        let lei = Lei::parse("5493001KJTIIGC8Y1R12").unwrap();
        assert_eq!(display(lei).as_str(), "5493001KJTIIGC8Y1R12");
    }

    #[test]
    fn as_ref_str() {
        let lei = Lei::parse("5493001KJTIIGC8Y1R12").unwrap();
        let s: &str = lei.as_ref();
        assert_eq!(s, "5493001KJTIIGC8Y1R12");
    }

    #[test]
    fn from_bytes_unchecked_round_trip() {
        let lei = Lei::from_bytes_unchecked(*b"5493001KJTIIGC8Y1R12");
        assert_eq!(lei, Lei::parse("5493001KJTIIGC8Y1R12").unwrap());
    }

    #[test]
    fn is_copy_and_eq_and_hashable() {
        let a = Lei::parse("5493001KJTIIGC8Y1R12").unwrap();
        let b = a; // Copy
        assert_eq!(a, b);
        assert_ne!(a, Lei::parse("549300DTUYXVMJXZNY75").unwrap());
        // Usable as a map key (Eq + Hash) — checked by constructing a slice.
        let keys = [a, b];
        assert_eq!(keys[0], keys[1]);
    }
}
