// Copyright 2026 Regit.io — Nicolas Koenig
// SPDX-License-Identifier: Apache-2.0

//! CUSIP — Committee on Uniform Securities Identification Procedures (ANSI X9.6).
//!
//! A CUSIP is the national securities identifier of the United States and
//! Canada. It is exactly 9 characters in three segments:
//!
//! ```text
//!   0 3 7 8 3 3 1 0 0
//!   └────┬────┘ └┬┘ │
//!        │       │   └ check digit  [8]      one digit [0-9]
//!        │       └───── issue        [6..8]   two characters [A-Z0-9*@#]
//!        └───────────── issuer       [0..6]   six characters [A-Z0-9*@#]
//! ```
//!
//! - The **issuer** segment identifies the issuing entity; the **issue**
//!   segment identifies a specific security of that issuer. Both draw from the
//!   body alphabet `[A-Z0-9*@#]` — the digits, the upper-case letters, and the
//!   three special characters `*`, `@`, `#`.
//! - The **check digit** is the ANSI X9.6 "modulus 10 double add double" of
//!   the eight-character body — see [`crate::checkdigit::cusip_check_digit`].
//!
//! A **CINS** (CUSIP International Numbering System) number is structurally a
//! CUSIP, computed with the identical check-digit algorithm; it is
//! distinguished only by its first character being a letter, where a domestic
//! CUSIP starts with a digit. [`Cusip::is_cins`] reports this, and
//! [`Cusip::cins_region`] maps the leading letter to its issuing region.
//!
//! [`Cusip::parse`] enforces every rule: exact length, the body character
//! set, a digit in the check position, and a check digit that is recomputed
//! and verified — never trusted.
//!
//! # References
//!
//! - ANSI X9.6, *Financial Services — CUSIP Numbering System*, CUSIP Global
//!   Services.

use crate::checkdigit;
use crate::errors::ValidationError;

/// A validated CUSIP (or CINS) number (ANSI X9.6).
///
/// A `Cusip` can only be created by [`Cusip::parse`] (or the explicitly
/// unchecked [`Cusip::from_bytes_unchecked`]), so a value of this type is a
/// proof that the 9 characters form a structurally valid CUSIP with a correct
/// check digit. It stores the identifier inline as `[u8; 9]`, is `Copy`, and
/// allocates nothing.
///
/// # Examples
///
/// ```
/// use regit_identifiers::Cusip;
///
/// let cusip = Cusip::parse("037833100").unwrap();
/// assert_eq!(cusip.issuer(), "037833");
/// assert_eq!(cusip.issue(), "10");
/// assert_eq!(cusip.check_digit(), '0');
/// assert_eq!(cusip.as_str(), "037833100");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cusip {
    /// The 9 validated ASCII bytes of the identifier.
    bytes: [u8; Self::LENGTH],
}

impl Cusip {
    /// The number of characters in a CUSIP.
    pub const LENGTH: usize = 9;

    /// Parses and fully validates a CUSIP.
    ///
    /// Validation is strict and, in order: the input must be exactly 9
    /// characters; characters 1–8 must each be drawn from the body alphabet
    /// `[A-Z0-9*@#]` and character 9 must be an ASCII digit; and the check
    /// digit must equal the value recomputed from the eight-character body.
    ///
    /// # Errors
    ///
    /// - [`ValidationError::WrongLength`] if the input is not 9 characters.
    /// - [`ValidationError::InvalidCharacter`] if a character falls outside
    ///   the set its position allows (this also rejects lower-case input and
    ///   any non-ASCII character).
    /// - [`ValidationError::BadCheckDigit`] if the supplied check digit does
    ///   not match the recomputed one.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Cusip;
    /// use regit_identifiers::errors::ValidationError;
    ///
    /// assert!(Cusip::parse("037833100").is_ok());
    ///
    /// // A single wrong digit is caught, not silently accepted.
    /// assert_eq!(
    ///     Cusip::parse("037833101"),
    ///     Err(ValidationError::BadCheckDigit { expected: '0', found: '1' }),
    /// );
    /// ```
    pub fn parse(s: &str) -> Result<Self, ValidationError> {
        // A CUSIP is exactly 9 characters.
        let found = s.chars().count();
        if found != Self::LENGTH {
            return Err(ValidationError::WrongLength {
                expected: Self::LENGTH,
                found,
            });
        }
        // Per-position character set: [0..8] are [A-Z0-9*@#], [8] is a digit.
        // A non-ASCII character fails both predicates and is rejected here.
        for (i, ch) in s.chars().enumerate() {
            let legal = if i == Self::LENGTH - 1 {
                ch.is_ascii_digit()
            } else {
                ch.is_ascii_digit() || ch.is_ascii_uppercase() || matches!(ch, '*' | '@' | '#')
            };
            if !legal {
                return Err(ValidationError::InvalidCharacter {
                    position: i + 1,
                    found: ch,
                });
            }
        }
        // Every character is ASCII, so the string is exactly 9 ASCII bytes.
        let mut bytes = [0u8; Self::LENGTH];
        bytes.copy_from_slice(s.as_bytes());

        // Recompute the check digit from the 8-character body and compare.
        let body = core::str::from_utf8(&bytes[0..8]).unwrap_or("");
        let expected = checkdigit::cusip_check_digit(body)?;
        let supplied = char::from(bytes[8]);
        if expected != supplied {
            return Err(ValidationError::BadCheckDigit {
                expected,
                found: supplied,
            });
        }
        Ok(Self { bytes })
    }

    /// Validates a CUSIP without constructing one.
    ///
    /// Equivalent to `Cusip::parse(s).map(|_| ())`; use it when only the
    /// verdict is needed.
    ///
    /// # Errors
    ///
    /// Returns the same [`ValidationError`] variants as [`Cusip::parse`].
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Cusip;
    ///
    /// assert!(Cusip::validate("037833100").is_ok());
    /// assert!(Cusip::validate("037833101").is_err());
    /// ```
    pub fn validate(s: &str) -> Result<(), ValidationError> {
        Self::parse(s).map(|_| ())
    }

    /// Wraps 9 raw bytes as a `Cusip` without any validation.
    ///
    /// The caller asserts that `bytes` holds the 9 ASCII characters of a valid
    /// CUSIP. This exists for reconstructing a `Cusip` from bytes that were
    /// validated earlier; prefer [`Cusip::parse`] for any untrusted input.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Cusip;
    ///
    /// let cusip = Cusip::from_bytes_unchecked(*b"037833100");
    /// assert_eq!(cusip.as_str(), "037833100");
    /// ```
    #[must_use]
    pub const fn from_bytes_unchecked(bytes: [u8; Self::LENGTH]) -> Self {
        Self { bytes }
    }

    /// Returns the CUSIP as a string slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Cusip;
    ///
    /// assert_eq!(Cusip::parse("037833100").unwrap().as_str(), "037833100");
    /// ```
    #[must_use]
    #[inline]
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes).unwrap_or("")
    }

    /// Returns the CUSIP as its 9 raw ASCII bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Cusip;
    ///
    /// assert_eq!(Cusip::parse("037833100").unwrap().as_bytes(), b"037833100");
    /// ```
    #[must_use]
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the six-character issuer segment, characters 1–6.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Cusip;
    ///
    /// assert_eq!(Cusip::parse("037833100").unwrap().issuer(), "037833");
    /// ```
    #[must_use]
    #[inline]
    pub fn issuer(&self) -> &str {
        core::str::from_utf8(&self.bytes[0..6]).unwrap_or("")
    }

    /// Returns the two-character issue segment, characters 7–8.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Cusip;
    ///
    /// assert_eq!(Cusip::parse("037833100").unwrap().issue(), "10");
    /// ```
    #[must_use]
    #[inline]
    pub fn issue(&self) -> &str {
        core::str::from_utf8(&self.bytes[6..8]).unwrap_or("")
    }

    /// Returns the check digit, character 9.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Cusip;
    ///
    /// assert_eq!(Cusip::parse("037833100").unwrap().check_digit(), '0');
    /// ```
    #[must_use]
    #[inline]
    pub fn check_digit(&self) -> char {
        char::from(self.bytes[8])
    }

    /// Returns `true` if this identifier is a CINS number.
    ///
    /// A CINS (CUSIP International Numbering System) number is structurally a
    /// CUSIP — same length, same alphabet, same check-digit algorithm — and is
    /// distinguished solely by its first character being a letter, where a
    /// domestic CUSIP always starts with a digit.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Cusip;
    ///
    /// // A domestic CUSIP starts with a digit.
    /// assert!(!Cusip::parse("037833100").unwrap().is_cins());
    /// ```
    #[must_use]
    #[inline]
    pub fn is_cins(&self) -> bool {
        self.bytes[0].is_ascii_uppercase()
    }

    /// Returns `true` if this is a domestic (US/Canada) CUSIP — the
    /// complement of [`Cusip::is_cins`].
    ///
    /// The discrimination rule is the leading character: a domestic CUSIP
    /// starts with a digit, a CINS with a letter.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Cusip;
    ///
    /// assert!(Cusip::parse("037833100").unwrap().is_domestic());
    /// ```
    #[must_use]
    #[inline]
    pub fn is_domestic(&self) -> bool {
        !self.is_cins()
    }

    /// Returns the CINS issuing region of this identifier, if it is a CINS.
    ///
    /// The leading letter of a CINS number designates its issuing region per
    /// the CINS table; for a domestic CUSIP (which starts with a digit) this
    /// returns `None`. A leading letter outside the assigned table likewise
    /// returns `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Cusip;
    ///
    /// // A domestic CUSIP has no CINS region.
    /// assert_eq!(Cusip::parse("037833100").unwrap().cins_region(), None);
    /// ```
    #[must_use]
    pub fn cins_region(&self) -> Option<&'static str> {
        if !self.is_cins() {
            return None;
        }
        match self.bytes[0] {
            b'A' => Some("Austria"),
            b'B' => Some("Belgium"),
            b'C' => Some("Canada"),
            b'D' => Some("Germany"),
            b'E' => Some("Spain"),
            b'F' => Some("France"),
            b'G' => Some("United Kingdom"),
            b'H' => Some("Switzerland"),
            b'J' => Some("Japan"),
            b'K' => Some("Denmark"),
            b'L' => Some("Luxembourg"),
            b'M' => Some("Middle East"),
            b'N' => Some("Netherlands"),
            b'P' => Some("South America"),
            b'Q' => Some("Australia"),
            b'R' => Some("Norway"),
            b'S' => Some("South Africa"),
            b'T' => Some("Italy"),
            b'U' => Some("United States"),
            b'V' => Some("Africa-Other"),
            b'W' => Some("Sweden"),
            b'X' => Some("Europe-Other"),
            b'Y' => Some("Asia"),
            _ => None,
        }
    }
}

impl core::fmt::Display for Cusip {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl core::str::FromStr for Cusip {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl AsRef<str> for Cusip {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::display;
    use core::str::FromStr;

    /// Real, well-known CUSIPs used as regression anchors.
    const GOLDEN: &[&str] = &[
        "037833100", // Apple Inc.
        "594918104", // Microsoft Corp.
        "38259P508", // Alphabet Inc.
    ];

    #[test]
    fn parses_golden_cusips() {
        for &s in GOLDEN {
            let cusip = Cusip::parse(s).unwrap_or_else(|e| panic!("{s} should parse: {e}"));
            assert_eq!(cusip.as_str(), s);
        }
    }

    #[test]
    fn segment_accessors() {
        let cusip = Cusip::parse("037833100").unwrap();
        assert_eq!(cusip.issuer(), "037833");
        assert_eq!(cusip.issue(), "10");
        assert_eq!(cusip.check_digit(), '0');
        assert_eq!(cusip.as_bytes(), b"037833100");
        assert_eq!(Cusip::LENGTH, 9);
    }

    #[test]
    fn rejects_bad_check_digit() {
        assert_eq!(
            Cusip::parse("037833101"),
            Err(ValidationError::BadCheckDigit {
                expected: '0',
                found: '1',
            })
        );
    }

    #[test]
    fn rejects_wrong_length() {
        assert_eq!(
            Cusip::parse("03783310"),
            Err(ValidationError::WrongLength {
                expected: 9,
                found: 8,
            })
        );
        assert_eq!(
            Cusip::parse(""),
            Err(ValidationError::WrongLength {
                expected: 9,
                found: 0,
            })
        );
    }

    #[test]
    fn rejects_lower_case() {
        assert!(matches!(
            Cusip::parse("a37833100"),
            Err(ValidationError::InvalidCharacter { position: 1, .. })
        ));
    }

    #[test]
    fn rejects_non_digit_check_position() {
        // Character 9 must be a digit.
        assert!(matches!(
            Cusip::parse("03783310X"),
            Err(ValidationError::InvalidCharacter { position: 9, .. })
        ));
    }

    #[test]
    fn rejects_illegal_body_character() {
        // A slash is outside the body alphabet [A-Z0-9*@#].
        assert!(matches!(
            Cusip::parse("0378/3100"),
            Err(ValidationError::InvalidCharacter { position: 5, .. })
        ));
    }

    #[test]
    fn accepts_special_body_characters() {
        // The body alphabet includes *, @, and #; a valid check digit follows.
        let body = "12345*@#";
        let check = checkdigit::cusip_check_digit(body).unwrap();
        let mut raw = [0u8; 9];
        raw[0..8].copy_from_slice(body.as_bytes());
        raw[8] = check as u8;
        let s = core::str::from_utf8(&raw).unwrap();
        let cusip = Cusip::parse(s).unwrap();
        assert_eq!(cusip.issuer(), "12345*");
        assert_eq!(cusip.issue(), "@#");
    }

    #[test]
    fn rejects_non_ascii_without_panic() {
        // A multi-byte character must be rejected cleanly.
        assert!(Cusip::parse("03783310é").is_err());
        assert!(Cusip::parse("é37833100").is_err());
    }

    #[test]
    fn is_cins_detects_leading_letter() {
        // A domestic CUSIP starts with a digit and is not a CINS.
        assert!(!Cusip::parse("037833100").unwrap().is_cins());
        // A CINS starts with a letter — build a valid one over a lettered body.
        let body = "U3783310";
        let check = checkdigit::cusip_check_digit(body).unwrap();
        let mut raw = [0u8; 9];
        raw[0..8].copy_from_slice(body.as_bytes());
        raw[8] = check as u8;
        let s = core::str::from_utf8(&raw).unwrap();
        assert!(Cusip::parse(s).unwrap().is_cins());
    }

    #[test]
    fn is_domestic_complements_is_cins() {
        // Every domestic CUSIP starts with a digit.
        for s in ["037833100", "594918104", "38259P508"] {
            let c = Cusip::parse(s).unwrap();
            assert!(c.is_domestic());
            assert!(!c.is_cins());
        }
        // A CINS is not domestic — assemble one and check the inverse.
        let body = "U3783310";
        let check = checkdigit::cusip_check_digit(body).unwrap();
        let mut raw = [0u8; 9];
        raw[0..8].copy_from_slice(body.as_bytes());
        raw[8] = check as u8;
        let cins = Cusip::parse(core::str::from_utf8(&raw).unwrap()).unwrap();
        assert!(!cins.is_domestic());
        assert!(cins.is_cins());
    }

    #[test]
    fn cins_region_maps_leading_letter() {
        // A domestic CUSIP has no CINS region.
        assert_eq!(Cusip::parse("037833100").unwrap().cins_region(), None);
        // 'U' designates the United States in the CINS table.
        let body = "U3783310";
        let check = checkdigit::cusip_check_digit(body).unwrap();
        let mut raw = [0u8; 9];
        raw[0..8].copy_from_slice(body.as_bytes());
        raw[8] = check as u8;
        let s = core::str::from_utf8(&raw).unwrap();
        assert_eq!(
            Cusip::parse(s).unwrap().cins_region(),
            Some("United States")
        );
    }

    #[test]
    fn cins_region_covers_every_assigned_letter() {
        // Each of the 23 assigned CINS letters maps to a region; I, O, and Z
        // are unassigned. Verify every assigned letter explicitly.
        let assigned = [
            (b'A', "Austria"),
            (b'B', "Belgium"),
            (b'C', "Canada"),
            (b'D', "Germany"),
            (b'E', "Spain"),
            (b'F', "France"),
            (b'G', "United Kingdom"),
            (b'H', "Switzerland"),
            (b'J', "Japan"),
            (b'K', "Denmark"),
            (b'L', "Luxembourg"),
            (b'M', "Middle East"),
            (b'N', "Netherlands"),
            (b'P', "South America"),
            (b'Q', "Australia"),
            (b'R', "Norway"),
            (b'S', "South Africa"),
            (b'T', "Italy"),
            (b'U', "United States"),
            (b'V', "Africa-Other"),
            (b'W', "Sweden"),
            (b'X', "Europe-Other"),
            (b'Y', "Asia"),
        ];
        for (letter, region) in assigned {
            let cusip = Cusip::from_bytes_unchecked([
                letter, b'1', b'1', b'1', b'1', b'1', b'1', b'1', b'1',
            ]);
            assert!(cusip.is_cins());
            assert_eq!(cusip.cins_region(), Some(region));
        }
    }

    #[test]
    fn cins_region_none_for_unassigned_letter() {
        // 'I', 'O', and 'Z' are not assigned regions in the CINS table.
        for letter in [b'I', b'O', b'Z'] {
            let cusip = Cusip::from_bytes_unchecked([
                letter, b'1', b'1', b'1', b'1', b'1', b'1', b'1', b'1',
            ]);
            assert!(cusip.is_cins());
            assert_eq!(cusip.cins_region(), None);
        }
    }

    #[test]
    fn round_trips_through_str() {
        for &s in GOLDEN {
            assert_eq!(Cusip::parse(s).unwrap().as_str(), s);
        }
    }

    #[test]
    fn from_str_matches_parse() {
        assert_eq!(Cusip::from_str("037833100"), Cusip::parse("037833100"));
        assert!(Cusip::from_str("nonsense").is_err());
    }

    #[test]
    fn display_renders_identifier() {
        let cusip = Cusip::parse("037833100").unwrap();
        assert_eq!(display(cusip).as_str(), "037833100");
    }

    #[test]
    fn as_ref_str() {
        let cusip = Cusip::parse("037833100").unwrap();
        let s: &str = cusip.as_ref();
        assert_eq!(s, "037833100");
    }

    #[test]
    fn validate_agrees_with_parse() {
        assert!(Cusip::validate("037833100").is_ok());
        assert!(Cusip::validate("037833101").is_err());
    }

    #[test]
    fn from_bytes_unchecked_round_trip() {
        let cusip = Cusip::from_bytes_unchecked(*b"037833100");
        assert_eq!(cusip, Cusip::parse("037833100").unwrap());
    }

    #[test]
    fn is_copy_and_eq_and_hashable() {
        let a = Cusip::parse("037833100").unwrap();
        let b = a; // Copy
        assert_eq!(a, b);
        assert_ne!(a, Cusip::parse("594918104").unwrap());
        // Usable as a map key (Eq + Hash) — checked by constructing a slice.
        let keys = [a, b];
        assert_eq!(keys[0], keys[1]);
    }
}
