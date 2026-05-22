// Copyright 2026 Regit.io — Nicolas Koenig
// SPDX-License-Identifier: Apache-2.0

//! ISIN — International Securities Identification Number (ISO 6166).
//!
//! An ISIN is the globally recognised primary key of a security. It is
//! exactly 12 characters in three segments:
//!
//! ```text
//!   U S 0 3 7 8 3 3 1 0 0 5
//!   └┬┘ └────┬────┘ │
//!    │       │       └ check digit  [11]      one digit [0-9]
//!    │       └───────── NSIN         [2..11]   nine characters [A-Z0-9]
//!    └───────────────── country      [0..2]    ISO 3166-1 or ISIN prefix
//! ```
//!
//! - The **country prefix** is an ISO 3166-1 alpha-2 code or one of the ISIN
//!   substitute prefixes (`XS`, `EU`, ...) — see [`crate::country`].
//! - The **NSIN** (National Securities Identifying Number) is the local
//!   identifier of the security, left-padded into nine characters.
//! - The **check digit** is the Luhn mod-10 over the expansion of the
//!   11-character body — see [`crate::checkdigit::isin_check_digit`].
//!
//! [`Isin::parse`] enforces every rule: exact length, the per-segment
//! character set, a recognised country prefix, and a check digit that is
//! recomputed and verified — never trusted.
//!
//! # References
//!
//! - ISO 6166, *Securities and related financial instruments —
//!   International securities identification number (ISIN)*.

use crate::checkdigit;
use crate::country;
use crate::errors::ValidationError;

/// A validated International Securities Identification Number (ISO 6166).
///
/// An `Isin` can only be created by [`Isin::parse`] (or the explicitly
/// unchecked [`Isin::from_bytes_unchecked`]), so a value of this type is a
/// proof that the 12 characters form a structurally valid ISIN with a
/// correct check digit. It stores the identifier inline as `[u8; 12]`, is
/// `Copy`, and allocates nothing.
///
/// # Examples
///
/// ```
/// use regit_identifiers::Isin;
///
/// let isin = Isin::parse("US0378331005").unwrap();
/// assert_eq!(isin.country_code(), "US");
/// assert_eq!(isin.nsin(), "037833100");
/// assert_eq!(isin.check_digit(), '5');
/// assert_eq!(isin.as_str(), "US0378331005");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Isin {
    /// The 12 validated ASCII bytes of the identifier.
    bytes: [u8; Self::LENGTH],
}

impl Isin {
    /// The number of characters in an ISIN.
    pub const LENGTH: usize = 12;

    /// Parses and fully validates an ISIN.
    ///
    /// Validation is strict and, in order: the input must be exactly 12
    /// characters; characters 1–11 must each be an ASCII digit or upper-case
    /// letter and character 12 an ASCII digit; the first two characters must
    /// be a recognised ISIN country prefix; and the check digit must equal
    /// the value recomputed from the 11-character body.
    ///
    /// # Errors
    ///
    /// - [`ValidationError::WrongLength`] if the input is not 12 characters.
    /// - [`ValidationError::InvalidCharacter`] if a character falls outside
    ///   the set its position allows (this also rejects lower-case input and
    ///   any non-ASCII character).
    /// - [`ValidationError::InvalidCountryCode`] if the first two characters
    ///   are not a recognised ISIN country prefix.
    /// - [`ValidationError::BadCheckDigit`] if the supplied check digit does
    ///   not match the recomputed one.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Isin;
    /// use regit_identifiers::errors::ValidationError;
    ///
    /// assert!(Isin::parse("US0378331005").is_ok());
    ///
    /// // A single wrong digit is caught, not silently accepted.
    /// assert_eq!(
    ///     Isin::parse("US0378331004"),
    ///     Err(ValidationError::BadCheckDigit { expected: '5', found: '4' }),
    /// );
    /// ```
    pub fn parse(s: &str) -> Result<Self, ValidationError> {
        // An ISIN is exactly 12 characters.
        let found = s.chars().count();
        if found != Self::LENGTH {
            return Err(ValidationError::WrongLength {
                expected: Self::LENGTH,
                found,
            });
        }
        // Per-position character set: [0..11] are [A-Z0-9], [11] is a digit.
        // A non-ASCII character fails both predicates and is rejected here.
        for (i, ch) in s.chars().enumerate() {
            let legal = if i == Self::LENGTH - 1 {
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
        // Every character is ASCII, so the string is exactly 12 ASCII bytes.
        let mut bytes = [0u8; Self::LENGTH];
        bytes.copy_from_slice(s.as_bytes());

        // The first two characters must be a recognised ISIN country prefix.
        let prefix = core::str::from_utf8(&bytes[0..2]).unwrap_or("");
        if !country::is_isin_prefix(prefix) {
            return Err(ValidationError::InvalidCountryCode);
        }
        // Recompute the check digit from the 11-character body and compare.
        let body = core::str::from_utf8(&bytes[0..11]).unwrap_or("");
        let expected = checkdigit::isin_check_digit(body)?;
        let supplied = char::from(bytes[11]);
        if expected != supplied {
            return Err(ValidationError::BadCheckDigit {
                expected,
                found: supplied,
            });
        }
        Ok(Self { bytes })
    }

    /// Validates an ISIN without constructing one.
    ///
    /// Equivalent to `Isin::parse(s).map(|_| ())`; use it when only the
    /// verdict is needed.
    ///
    /// # Errors
    ///
    /// Returns the same [`ValidationError`] variants as [`Isin::parse`].
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Isin;
    ///
    /// assert!(Isin::validate("US0378331005").is_ok());
    /// assert!(Isin::validate("US0378331004").is_err());
    /// ```
    pub fn validate(s: &str) -> Result<(), ValidationError> {
        Self::parse(s).map(|_| ())
    }

    /// Wraps 12 raw bytes as an `Isin` without any validation.
    ///
    /// The caller asserts that `bytes` holds the 12 ASCII characters of a
    /// valid ISIN. This exists for reconstructing an `Isin` from bytes that
    /// were validated earlier; prefer [`Isin::parse`] for any untrusted
    /// input.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Isin;
    ///
    /// let isin = Isin::from_bytes_unchecked(*b"US0378331005");
    /// assert_eq!(isin.as_str(), "US0378331005");
    /// ```
    #[must_use]
    pub const fn from_bytes_unchecked(bytes: [u8; Self::LENGTH]) -> Self {
        Self { bytes }
    }

    /// Returns the ISIN as a string slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Isin;
    ///
    /// assert_eq!(Isin::parse("US0378331005").unwrap().as_str(), "US0378331005");
    /// ```
    #[must_use]
    #[inline]
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes).unwrap_or("")
    }

    /// Returns the ISIN as its 12 raw ASCII bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Isin;
    ///
    /// assert_eq!(Isin::parse("US0378331005").unwrap().as_bytes(), b"US0378331005");
    /// ```
    #[must_use]
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the two-character country prefix, characters 1–2.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Isin;
    ///
    /// assert_eq!(Isin::parse("US0378331005").unwrap().country_code(), "US");
    /// ```
    #[must_use]
    #[inline]
    pub fn country_code(&self) -> &str {
        core::str::from_utf8(&self.bytes[0..2]).unwrap_or("")
    }

    /// Returns the nine-character NSIN, characters 3–11.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Isin;
    ///
    /// assert_eq!(Isin::parse("US0378331005").unwrap().nsin(), "037833100");
    /// ```
    #[must_use]
    #[inline]
    pub fn nsin(&self) -> &str {
        core::str::from_utf8(&self.bytes[2..11]).unwrap_or("")
    }

    /// Returns the check digit, character 12.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Isin;
    ///
    /// assert_eq!(Isin::parse("US0378331005").unwrap().check_digit(), '5');
    /// ```
    #[must_use]
    #[inline]
    pub fn check_digit(&self) -> char {
        char::from(self.bytes[11])
    }
}

impl core::fmt::Display for Isin {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl core::str::FromStr for Isin {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl AsRef<str> for Isin {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::display;
    use core::str::FromStr;

    /// Real, well-known ISINs used as regression anchors.
    const GOLDEN: &[&str] = &[
        "US0378331005", // Apple Inc.
        "US5949181045", // Microsoft Corp.
        "GB0002634946", // BAE Systems plc
        "DE000BAY0017", // Bayer AG
        "FR0000131104", // BNP Paribas
        "NL0011794037", // ABN AMRO
    ];

    #[test]
    fn parses_golden_isins() {
        for &s in GOLDEN {
            let isin = Isin::parse(s).unwrap_or_else(|e| panic!("{s} should parse: {e}"));
            assert_eq!(isin.as_str(), s);
        }
    }

    #[test]
    fn segment_accessors() {
        let isin = Isin::parse("US0378331005").unwrap();
        assert_eq!(isin.country_code(), "US");
        assert_eq!(isin.nsin(), "037833100");
        assert_eq!(isin.check_digit(), '5');
        assert_eq!(isin.as_bytes(), b"US0378331005");
        assert_eq!(Isin::LENGTH, 12);
    }

    #[test]
    fn accepts_substitute_prefix() {
        // XS is an ISIN substitute prefix, not an ISO country code.
        let isin = Isin::parse("XS0000000009").unwrap();
        assert_eq!(isin.country_code(), "XS");
    }

    #[test]
    fn rejects_bad_check_digit() {
        assert_eq!(
            Isin::parse("US0378331004"),
            Err(ValidationError::BadCheckDigit {
                expected: '5',
                found: '4',
            })
        );
    }

    #[test]
    fn rejects_wrong_length() {
        assert_eq!(
            Isin::parse("US037833100"),
            Err(ValidationError::WrongLength {
                expected: 12,
                found: 11,
            })
        );
        assert_eq!(
            Isin::parse(""),
            Err(ValidationError::WrongLength {
                expected: 12,
                found: 0,
            })
        );
    }

    #[test]
    fn rejects_lower_case() {
        assert!(matches!(
            Isin::parse("us0378331005"),
            Err(ValidationError::InvalidCharacter { position: 1, .. })
        ));
    }

    #[test]
    fn rejects_non_digit_check_position() {
        // Character 12 must be a digit.
        assert!(matches!(
            Isin::parse("US037833100X"),
            Err(ValidationError::InvalidCharacter { position: 12, .. })
        ));
    }

    #[test]
    fn rejects_unknown_country_code() {
        assert_eq!(
            Isin::parse("ZZ0378331005"),
            Err(ValidationError::InvalidCountryCode)
        );
    }

    #[test]
    fn rejects_non_ascii_without_panic() {
        // A multi-byte character must be rejected cleanly.
        assert!(Isin::parse("US037833100é").is_err());
        assert!(Isin::parse("ÉS0378331005").is_err());
    }

    #[test]
    fn round_trips_through_str() {
        for &s in GOLDEN {
            assert_eq!(Isin::parse(s).unwrap().as_str(), s);
        }
    }

    #[test]
    fn from_str_matches_parse() {
        assert_eq!(Isin::from_str("US0378331005"), Isin::parse("US0378331005"));
        assert!(Isin::from_str("nonsense").is_err());
    }

    #[test]
    fn display_renders_identifier() {
        let isin = Isin::parse("US0378331005").unwrap();
        assert_eq!(display(isin).as_str(), "US0378331005");
    }

    #[test]
    fn as_ref_str() {
        let isin = Isin::parse("US0378331005").unwrap();
        let s: &str = isin.as_ref();
        assert_eq!(s, "US0378331005");
    }

    #[test]
    fn from_bytes_unchecked_round_trip() {
        let isin = Isin::from_bytes_unchecked(*b"US0378331005");
        assert_eq!(isin, Isin::parse("US0378331005").unwrap());
    }

    #[test]
    fn is_copy_and_eq_and_hashable() {
        let a = Isin::parse("US0378331005").unwrap();
        let b = a; // Copy
        assert_eq!(a, b);
        assert_ne!(a, Isin::parse("US5949181045").unwrap());
        // Usable as a map key (Eq + Hash) — checked by constructing a slice.
        let keys = [a, b];
        assert_eq!(keys[0], keys[1]);
    }
}
