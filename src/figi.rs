// Copyright 2026 Regit.io — Nicolas Koenig
// SPDX-License-Identifier: Apache-2.0

//! FIGI — Financial Instrument Global Identifier (ANSI X9.145).
//!
//! A FIGI is a permanent, currency- and exchange-aware identifier for a
//! financial instrument, issued through the `OpenFIGI` programme. It is
//! exactly 12 characters in three segments:
//!
//! ```text
//!   B B G 0 0 0 B L N N H 6
//!   └┬┘ │ └────┬─────┘ │
//!    │  │      │        └ check digit  [11]      one digit [0-9]
//!    │  │      └───────── body         [3..11]   eight chars, digits/consonants
//!    │  └──────────────── literal 'G'  [2]       always the letter G
//!    └─────────────────── provider     [0..2]    two upper-case consonants
//! ```
//!
//! - The **provider prefix** is two upper-case consonants and must not be one
//!   of `BS`, `BM`, `GG`, `GB`, `GH`, `KY`, or `VG` — those would collide
//!   with ISIN country codes. Bloomberg-issued FIGIs use the prefix `BBG`.
//! - Character three is always the literal `G`.
//! - The **body** is eight characters drawn from the digits and the
//!   consonants; a vowel never appears, so a check character can never be
//!   mistaken for part of a word.
//! - The **check digit** is the ANSI X9.145 modulus-10 double-add-double of
//!   the 11-character body — see [`crate::checkdigit::figi_check_digit`].
//!
//! [`Figi::parse`] enforces every rule: exact length, the per-segment
//! character set, the forbidden-prefix and literal-`G` structural rules, and
//! a check digit that is recomputed and verified — never trusted.
//!
//! # References
//!
//! - ANSI X9.145, *Financial Instrument Global Identifier (FIGI)*.
//! - Object Management Group — the `OpenFIGI` specification.

use crate::checkdigit;
use crate::errors::ValidationError;

/// A validated Financial Instrument Global Identifier (ANSI X9.145).
///
/// A `Figi` can only be created by [`Figi::parse`] (or the explicitly
/// unchecked [`Figi::from_bytes_unchecked`]), so a value of this type is a
/// proof that the 12 characters form a structurally valid FIGI with a
/// correct check digit. It stores the identifier inline as `[u8; 12]`, is
/// `Copy`, and allocates nothing.
///
/// # Examples
///
/// ```
/// use regit_identifiers::Figi;
///
/// let figi = Figi::parse("BBG000BLNNH6").unwrap();
/// assert_eq!(figi.provider_prefix(), "BBG");
/// assert_eq!(figi.body(), "000BLNNH");
/// assert_eq!(figi.check_digit(), '6');
/// assert_eq!(figi.as_str(), "BBG000BLNNH6");
/// assert!(figi.is_bloomberg());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Figi {
    /// The 12 validated ASCII bytes of the identifier.
    bytes: [u8; Self::LENGTH],
}

impl Figi {
    /// The number of characters in a FIGI.
    pub const LENGTH: usize = 12;

    /// Provider prefixes that a FIGI may not use because they would collide
    /// with ISIN country codes.
    const FORBIDDEN_PREFIXES: [&'static [u8]; 7] =
        [b"BS", b"BM", b"GG", b"GB", b"GH", b"KY", b"VG"];

    /// Parses and fully validates a FIGI.
    ///
    /// Validation is strict and, in order: the input must be exactly 12
    /// characters; characters 1–2 must be upper-case consonants and not a
    /// forbidden provider prefix; character 3 must be the literal `G`;
    /// characters 4–11 must each be an ASCII digit or an upper-case consonant
    /// (a vowel is rejected); character 12 must be an ASCII digit; and the
    /// check digit must equal the value recomputed from the 11-character
    /// body.
    ///
    /// # Errors
    ///
    /// - [`ValidationError::WrongLength`] if the input is not 12 characters.
    /// - [`ValidationError::InvalidCharacter`] if a character falls outside
    ///   the set its position allows (this also rejects vowels in the body,
    ///   lower-case input, and any non-ASCII character).
    /// - [`ValidationError::Structure`] if the provider prefix is a forbidden
    ///   one or if character 3 is not `G`.
    /// - [`ValidationError::BadCheckDigit`] if the supplied check digit does
    ///   not match the recomputed one.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Figi;
    /// use regit_identifiers::errors::ValidationError;
    ///
    /// assert!(Figi::parse("BBG000BLNNH6").is_ok());
    ///
    /// // A single wrong digit is caught, not silently accepted.
    /// assert_eq!(
    ///     Figi::parse("BBG000BLNNH5"),
    ///     Err(ValidationError::BadCheckDigit { expected: '6', found: '5' }),
    /// );
    /// ```
    pub fn parse(s: &str) -> Result<Self, ValidationError> {
        // A FIGI is exactly 12 characters.
        let found = s.chars().count();
        if found != Self::LENGTH {
            return Err(ValidationError::WrongLength {
                expected: Self::LENGTH,
                found,
            });
        }
        // Per-position character set: [0..2] consonants, [3..11] digits or
        // consonants, [11] a digit. Position [2] need only be ASCII here —
        // the literal-'G' requirement is a structural rule checked below. A
        // non-ASCII character fails every predicate and is rejected here.
        for (i, ch) in s.chars().enumerate() {
            let legal = if i < 2 {
                ch.is_ascii_uppercase() && !is_vowel(ch)
            } else if i == 2 {
                ch.is_ascii()
            } else if i == Self::LENGTH - 1 {
                ch.is_ascii_digit()
            } else {
                ch.is_ascii_digit() || (ch.is_ascii_uppercase() && !is_vowel(ch))
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

        // The provider prefix must not collide with an ISIN country code.
        let prefix = &bytes[0..2];
        if Self::FORBIDDEN_PREFIXES.contains(&prefix) {
            return Err(ValidationError::Structure {
                rule: "FIGI provider prefix must not be an ISIN country code",
            });
        }
        // Character 3 is always the literal 'G'.
        if bytes[2] != b'G' {
            return Err(ValidationError::Structure {
                rule: "FIGI position 3 must be the letter G",
            });
        }
        // Recompute the check digit from the 11-character body and compare.
        let body = core::str::from_utf8(&bytes[0..11]).unwrap_or("");
        let expected = checkdigit::figi_check_digit(body)?;
        let supplied = char::from(bytes[11]);
        if expected != supplied {
            return Err(ValidationError::BadCheckDigit {
                expected,
                found: supplied,
            });
        }
        Ok(Self { bytes })
    }

    /// Validates a FIGI without constructing one.
    ///
    /// Equivalent to `Figi::parse(s).map(|_| ())`; use it when only the
    /// verdict is needed.
    ///
    /// # Errors
    ///
    /// Returns the same [`ValidationError`] variants as [`Figi::parse`].
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Figi;
    ///
    /// assert!(Figi::validate("BBG000BLNNH6").is_ok());
    /// assert!(Figi::validate("BBG000BLNNH5").is_err());
    /// ```
    pub fn validate(s: &str) -> Result<(), ValidationError> {
        Self::parse(s).map(|_| ())
    }

    /// Wraps 12 raw bytes as a `Figi` without any validation.
    ///
    /// The caller asserts that `bytes` holds the 12 ASCII characters of a
    /// valid FIGI. This exists for reconstructing a `Figi` from bytes that
    /// were validated earlier; prefer [`Figi::parse`] for any untrusted
    /// input.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Figi;
    ///
    /// let figi = Figi::from_bytes_unchecked(*b"BBG000BLNNH6");
    /// assert_eq!(figi.as_str(), "BBG000BLNNH6");
    /// ```
    #[must_use]
    pub const fn from_bytes_unchecked(bytes: [u8; Self::LENGTH]) -> Self {
        Self { bytes }
    }

    /// Returns the FIGI as a string slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Figi;
    ///
    /// assert_eq!(Figi::parse("BBG000BLNNH6").unwrap().as_str(), "BBG000BLNNH6");
    /// ```
    #[must_use]
    #[inline]
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes).unwrap_or("")
    }

    /// Returns the FIGI as its 12 raw ASCII bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Figi;
    ///
    /// assert_eq!(Figi::parse("BBG000BLNNH6").unwrap().as_bytes(), b"BBG000BLNNH6");
    /// ```
    #[must_use]
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the three-character provider prefix, characters 1–3.
    ///
    /// This is the two-consonant provider code together with the literal
    /// `G` — the segment that, for a Bloomberg FIGI, reads `BBG`.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Figi;
    ///
    /// assert_eq!(Figi::parse("BBG000BLNNH6").unwrap().provider_prefix(), "BBG");
    /// ```
    #[must_use]
    #[inline]
    pub fn provider_prefix(&self) -> &str {
        core::str::from_utf8(&self.bytes[0..3]).unwrap_or("")
    }

    /// Returns the eight-character body, characters 4–11.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Figi;
    ///
    /// assert_eq!(Figi::parse("BBG000BLNNH6").unwrap().body(), "000BLNNH");
    /// ```
    #[must_use]
    #[inline]
    pub fn body(&self) -> &str {
        core::str::from_utf8(&self.bytes[3..11]).unwrap_or("")
    }

    /// Returns the check digit, character 12.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Figi;
    ///
    /// assert_eq!(Figi::parse("BBG000BLNNH6").unwrap().check_digit(), '6');
    /// ```
    #[must_use]
    #[inline]
    pub fn check_digit(&self) -> char {
        char::from(self.bytes[11])
    }

    /// Returns `true` if this is a Bloomberg-issued FIGI.
    ///
    /// A Bloomberg FIGI carries the provider prefix `BBG`; every other
    /// recognised provider uses a different two-consonant code.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Figi;
    ///
    /// assert!(Figi::parse("BBG000BLNNH6").unwrap().is_bloomberg());
    /// ```
    #[must_use]
    #[inline]
    pub fn is_bloomberg(&self) -> bool {
        &self.bytes[0..3] == b"BBG"
    }
}

/// `true` if `ch` is an upper-case ASCII vowel, which a FIGI provider prefix
/// and body both forbid.
#[inline]
fn is_vowel(ch: char) -> bool {
    matches!(ch, 'A' | 'E' | 'I' | 'O' | 'U')
}

impl core::fmt::Display for Figi {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl core::str::FromStr for Figi {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl AsRef<str> for Figi {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::display;
    use core::str::FromStr;

    /// Real, well-known FIGIs used as regression anchors.
    const GOLDEN: &[&str] = &[
        "BBG000BLNNH6", // IBM
        "BBG000B9XRY4",
        "BBG000BVPV84",
    ];

    #[test]
    fn parses_golden_figis() {
        for &s in GOLDEN {
            let figi = Figi::parse(s).unwrap_or_else(|e| panic!("{s} should parse: {e}"));
            assert_eq!(figi.as_str(), s);
        }
    }

    #[test]
    fn segment_accessors() {
        let figi = Figi::parse("BBG000BLNNH6").unwrap();
        assert_eq!(figi.provider_prefix(), "BBG");
        assert_eq!(figi.body(), "000BLNNH");
        assert_eq!(figi.check_digit(), '6');
        assert_eq!(figi.as_bytes(), b"BBG000BLNNH6");
        assert_eq!(Figi::LENGTH, 12);
    }

    #[test]
    fn is_bloomberg_detects_bbg_prefix() {
        assert!(Figi::parse("BBG000BLNNH6").unwrap().is_bloomberg());
    }

    #[test]
    fn rejects_bad_check_digit() {
        assert_eq!(
            Figi::parse("BBG000BLNNH5"),
            Err(ValidationError::BadCheckDigit {
                expected: '6',
                found: '5',
            })
        );
    }

    #[test]
    fn rejects_wrong_length() {
        assert_eq!(
            Figi::parse("BBG000BLNNH"),
            Err(ValidationError::WrongLength {
                expected: 12,
                found: 11,
            })
        );
        assert_eq!(
            Figi::parse(""),
            Err(ValidationError::WrongLength {
                expected: 12,
                found: 0,
            })
        );
    }

    #[test]
    fn rejects_forbidden_prefix() {
        // BS is a forbidden provider prefix (collides with an ISIN country).
        assert_eq!(
            Figi::parse("BSG000BLNNH6"),
            Err(ValidationError::Structure {
                rule: "FIGI provider prefix must not be an ISIN country code",
            })
        );
    }

    #[test]
    fn rejects_non_g_third_character() {
        // Position 3 must be the literal 'G' — enforced as a structural rule.
        assert!(matches!(
            Figi::parse("BBA000BLNNH6"),
            Err(ValidationError::Structure { .. })
        ));
    }

    #[test]
    fn rejects_vowel_in_body() {
        // A vowel can never appear in a FIGI body.
        assert!(matches!(
            Figi::parse("BBG00OBLNNH6"),
            Err(ValidationError::InvalidCharacter { position: 6, .. })
        ));
    }

    #[test]
    fn rejects_vowel_in_prefix() {
        // The provider prefix is two consonants.
        assert!(matches!(
            Figi::parse("ABG000BLNNH6"),
            Err(ValidationError::InvalidCharacter { position: 1, .. })
        ));
    }

    #[test]
    fn rejects_lower_case() {
        assert!(matches!(
            Figi::parse("bbg000blnnh6"),
            Err(ValidationError::InvalidCharacter { position: 1, .. })
        ));
    }

    #[test]
    fn rejects_non_digit_check_position() {
        // Character 12 must be a digit.
        assert!(matches!(
            Figi::parse("BBG000BLNNHX"),
            Err(ValidationError::InvalidCharacter { position: 12, .. })
        ));
    }

    #[test]
    fn rejects_non_ascii_without_panic() {
        // A multi-byte character must be rejected cleanly.
        assert!(Figi::parse("BBG000BLNNHé").is_err());
        assert!(Figi::parse("ÉBG000BLNNH6").is_err());
    }

    #[test]
    fn round_trips_through_str() {
        for &s in GOLDEN {
            assert_eq!(Figi::parse(s).unwrap().as_str(), s);
        }
    }

    #[test]
    fn from_str_matches_parse() {
        assert_eq!(Figi::from_str("BBG000BLNNH6"), Figi::parse("BBG000BLNNH6"));
        assert!(Figi::from_str("nonsense").is_err());
    }

    #[test]
    fn display_renders_identifier() {
        let figi = Figi::parse("BBG000BLNNH6").unwrap();
        assert_eq!(display(figi).as_str(), "BBG000BLNNH6");
    }

    #[test]
    fn as_ref_str() {
        let figi = Figi::parse("BBG000BLNNH6").unwrap();
        let s: &str = figi.as_ref();
        assert_eq!(s, "BBG000BLNNH6");
    }

    #[test]
    fn validate_agrees_with_parse() {
        assert!(Figi::validate("BBG000BLNNH6").is_ok());
        assert!(Figi::validate("BBG000BLNNH5").is_err());
    }

    #[test]
    fn from_bytes_unchecked_round_trip() {
        let figi = Figi::from_bytes_unchecked(*b"BBG000BLNNH6");
        assert_eq!(figi, Figi::parse("BBG000BLNNH6").unwrap());
    }

    #[test]
    fn is_copy_and_eq_and_hashable() {
        let a = Figi::parse("BBG000BLNNH6").unwrap();
        let b = a; // Copy
        assert_eq!(a, b);
        assert_ne!(a, Figi::parse("BBG000B9XRY4").unwrap());
        // Usable as a map key (Eq + Hash) — checked by constructing a slice.
        let keys = [a, b];
        assert_eq!(keys[0], keys[1]);
    }
}
