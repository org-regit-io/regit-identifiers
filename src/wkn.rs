// Copyright 2026 Regit.io — Nicolas Koenig
// SPDX-License-Identifier: Apache-2.0

//! WKN — Wertpapierkennnummer (WM Datenservice).
//!
//! A WKN is the German national securities identifying number. It is exactly
//! 6 characters with no internal structure:
//!
//! ```text
//!   A 1 E W W W
//!   └─────┬─────┘
//!         └ identifier  [0..6]  six characters [0-9A-Z], excluding I and O
//! ```
//!
//! - Each character is an ASCII digit or an upper-case letter, with the two
//!   letters `I` and `O` **excluded** — they are barred to avoid visual
//!   confusion with the digits `1` and `0`.
//! - A WKN has **no segments and no check digit**: validation is purely a
//!   length and character-set check.
//!
//! [`Wkn::parse`] enforces every rule: the exact length and the per-character
//! set, rejecting a literal `I` or `O` as an invalid character.
//!
//! # References
//!
//! - WM Datenservice, *Wertpapierkennnummer (WKN)* — the German national
//!   securities-numbering scheme.

use crate::errors::ValidationError;

/// A validated Wertpapierkennnummer (WKN).
///
/// A `Wkn` can only be created by [`Wkn::parse`] (or the explicitly unchecked
/// [`Wkn::from_bytes_unchecked`]), so a value of this type is a proof that the
/// six characters form a structurally valid WKN. It stores the identifier
/// inline as `[u8; 6]`, is `Copy`, and allocates nothing.
///
/// # Examples
///
/// ```
/// use regit_identifiers::Wkn;
///
/// let wkn = Wkn::parse("A1EWWW").unwrap();
/// assert_eq!(wkn.as_str(), "A1EWWW");
/// assert!(!wkn.is_numeric());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Wkn {
    /// The 6 validated ASCII bytes of the identifier.
    bytes: [u8; Self::LENGTH],
}

impl Wkn {
    /// The number of characters in a WKN.
    pub const LENGTH: usize = 6;

    /// Parses and fully validates a WKN.
    ///
    /// Validation is strict and, in order: the input must be exactly 6
    /// characters; each character must be an ASCII digit or an upper-case
    /// letter, with `I` and `O` excluded. A WKN has no check digit, so a
    /// structurally valid string is always accepted.
    ///
    /// # Errors
    ///
    /// - [`ValidationError::WrongLength`] if the input is not 6 characters.
    /// - [`ValidationError::InvalidCharacter`] if a character is not an ASCII
    ///   digit or upper-case letter, or is a literal `I` or `O` (this also
    ///   rejects lower-case input and any non-ASCII character).
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Wkn;
    /// use regit_identifiers::errors::ValidationError;
    ///
    /// assert!(Wkn::parse("766403").is_ok());
    ///
    /// // A literal `I` is rejected, not silently accepted.
    /// assert_eq!(
    ///     Wkn::parse("A1IWWW"),
    ///     Err(ValidationError::InvalidCharacter { position: 3, found: 'I' }),
    /// );
    /// ```
    pub fn parse(s: &str) -> Result<Self, ValidationError> {
        // A WKN is exactly 6 characters.
        let found = s.chars().count();
        if found != Self::LENGTH {
            return Err(ValidationError::WrongLength {
                expected: Self::LENGTH,
                found,
            });
        }
        // Per-character set: an ASCII digit or upper-case letter, excluding the
        // letters `I` and `O`. A non-ASCII character fails the predicate and is
        // rejected here.
        for (i, ch) in s.chars().enumerate() {
            let legal = (ch.is_ascii_digit() || ch.is_ascii_uppercase()) && ch != 'I' && ch != 'O';
            if !legal {
                return Err(ValidationError::InvalidCharacter {
                    position: i + 1,
                    found: ch,
                });
            }
        }
        // Every character is ASCII, so the string is exactly 6 ASCII bytes.
        let mut bytes = [0u8; Self::LENGTH];
        bytes.copy_from_slice(s.as_bytes());
        Ok(Self { bytes })
    }

    /// Validates a WKN without constructing one.
    ///
    /// Equivalent to `Wkn::parse(s).map(|_| ())`; use it when only the verdict
    /// is needed.
    ///
    /// # Errors
    ///
    /// Returns the same [`ValidationError`] variants as [`Wkn::parse`].
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Wkn;
    ///
    /// assert!(Wkn::validate("519000").is_ok());
    /// assert!(Wkn::validate("A1IWWW").is_err());
    /// ```
    pub fn validate(s: &str) -> Result<(), ValidationError> {
        Self::parse(s).map(|_| ())
    }

    /// Wraps 6 raw bytes as a `Wkn` without any validation.
    ///
    /// The caller asserts that `bytes` holds the 6 ASCII characters of a valid
    /// WKN. This exists for reconstructing a `Wkn` from bytes that were
    /// validated earlier; prefer [`Wkn::parse`] for any untrusted input.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Wkn;
    ///
    /// let wkn = Wkn::from_bytes_unchecked(*b"A1EWWW");
    /// assert_eq!(wkn.as_str(), "A1EWWW");
    /// ```
    #[must_use]
    pub const fn from_bytes_unchecked(bytes: [u8; Self::LENGTH]) -> Self {
        Self { bytes }
    }

    /// Returns the WKN as a string slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Wkn;
    ///
    /// assert_eq!(Wkn::parse("766403").unwrap().as_str(), "766403");
    /// ```
    #[must_use]
    #[inline]
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes).unwrap_or("")
    }

    /// Returns the WKN as its 6 raw ASCII bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Wkn;
    ///
    /// assert_eq!(Wkn::parse("766403").unwrap().as_bytes(), b"766403");
    /// ```
    #[must_use]
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns `true` if all six characters are ASCII digits.
    ///
    /// A purely numeric WKN is a legacy identifier; alphanumeric WKNs were
    /// introduced once the numeric space began to run out.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Wkn;
    ///
    /// assert!(Wkn::parse("766403").unwrap().is_numeric());
    /// assert!(!Wkn::parse("A1EWWW").unwrap().is_numeric());
    /// ```
    #[must_use]
    #[inline]
    pub fn is_numeric(&self) -> bool {
        self.bytes.iter().all(u8::is_ascii_digit)
    }
}

impl core::fmt::Display for Wkn {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl core::str::FromStr for Wkn {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl AsRef<str> for Wkn {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::display;
    use core::str::FromStr;

    /// Real, well-known WKNs used as regression anchors.
    const GOLDEN: &[&str] = &[
        "766403", // Volkswagen AG
        "519000", // Bayerische Motoren Werke AG
        "A1EWWW", // Adidas AG
    ];

    #[test]
    fn parses_golden_wkns() {
        for &s in GOLDEN {
            let wkn = Wkn::parse(s).unwrap_or_else(|e| panic!("{s} should parse: {e}"));
            assert_eq!(wkn.as_str(), s);
        }
    }

    #[test]
    fn accessors() {
        let wkn = Wkn::parse("A1EWWW").unwrap();
        assert_eq!(wkn.as_str(), "A1EWWW");
        assert_eq!(wkn.as_bytes(), b"A1EWWW");
        assert_eq!(Wkn::LENGTH, 6);
    }

    #[test]
    fn is_numeric_classifies() {
        assert!(Wkn::parse("766403").unwrap().is_numeric());
        assert!(Wkn::parse("519000").unwrap().is_numeric());
        assert!(!Wkn::parse("A1EWWW").unwrap().is_numeric());
        assert!(!Wkn::parse("ABCDEF").unwrap().is_numeric());
    }

    #[test]
    fn rejects_letter_i() {
        assert_eq!(
            Wkn::parse("A1IWWW"),
            Err(ValidationError::InvalidCharacter {
                position: 3,
                found: 'I',
            })
        );
    }

    #[test]
    fn rejects_letter_o() {
        assert_eq!(
            Wkn::parse("A1OWWW"),
            Err(ValidationError::InvalidCharacter {
                position: 3,
                found: 'O',
            })
        );
    }

    #[test]
    fn rejects_wrong_length() {
        assert_eq!(
            Wkn::parse("76640"),
            Err(ValidationError::WrongLength {
                expected: 6,
                found: 5,
            })
        );
        assert_eq!(
            Wkn::parse("7664033"),
            Err(ValidationError::WrongLength {
                expected: 6,
                found: 7,
            })
        );
        assert_eq!(
            Wkn::parse(""),
            Err(ValidationError::WrongLength {
                expected: 6,
                found: 0,
            })
        );
    }

    #[test]
    fn rejects_lower_case() {
        assert!(matches!(
            Wkn::parse("a1ewww"),
            Err(ValidationError::InvalidCharacter { position: 1, .. })
        ));
    }

    #[test]
    fn rejects_punctuation() {
        assert!(matches!(
            Wkn::parse("A1-WWW"),
            Err(ValidationError::InvalidCharacter {
                position: 3,
                found: '-',
            })
        ));
    }

    #[test]
    fn rejects_non_ascii_without_panic() {
        // A multi-byte character must be rejected cleanly.
        assert!(Wkn::parse("A1EWWé").is_err());
        assert!(Wkn::parse("É1EWWW").is_err());
    }

    #[test]
    fn round_trips_through_str() {
        for &s in GOLDEN {
            assert_eq!(Wkn::parse(s).unwrap().as_str(), s);
        }
    }

    #[test]
    fn from_str_matches_parse() {
        assert_eq!(Wkn::from_str("A1EWWW"), Wkn::parse("A1EWWW"));
        assert!(Wkn::from_str("nonsense").is_err());
    }

    #[test]
    fn display_renders_identifier() {
        let wkn = Wkn::parse("A1EWWW").unwrap();
        assert_eq!(display(wkn).as_str(), "A1EWWW");
    }

    #[test]
    fn as_ref_str() {
        let wkn = Wkn::parse("766403").unwrap();
        let s: &str = wkn.as_ref();
        assert_eq!(s, "766403");
    }

    #[test]
    fn validate_matches_parse() {
        assert!(Wkn::validate("519000").is_ok());
        assert!(Wkn::validate("A1IWWW").is_err());
    }

    #[test]
    fn from_bytes_unchecked_round_trip() {
        let wkn = Wkn::from_bytes_unchecked(*b"A1EWWW");
        assert_eq!(wkn, Wkn::parse("A1EWWW").unwrap());
    }

    #[test]
    fn is_copy_and_eq_and_hashable() {
        let a = Wkn::parse("A1EWWW").unwrap();
        let b = a; // Copy
        assert_eq!(a, b);
        assert_ne!(a, Wkn::parse("766403").unwrap());
        // Usable as a map key (Eq + Hash) — checked by constructing a slice.
        let keys = [a, b];
        assert_eq!(keys[0], keys[1]);
    }
}
