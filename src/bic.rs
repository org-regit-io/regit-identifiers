// Copyright 2026 Regit.io — Nicolas Koenig
// SPDX-License-Identifier: Apache-2.0

//! BIC — Business Identifier Code (ISO 9362), the SWIFT address of a
//! financial institution.
//!
//! A BIC identifies a bank or other institution on the SWIFT network. It is
//! either **8** characters (an institution's primary office) or **11**
//! characters (the same with an explicit branch suffix), in up to four
//! segments:
//!
//! ```text
//!   D E U T D E F F 5 0 0
//!   └──┬──┘ └┬┘ └┬┘ └─┬─┘
//!     │      │   │     └ branch       [8..11]  three [A-Z0-9], 11-char only
//!     │      │   └─────── location    [6..8]   two [A-Z0-9]
//!     │      └─────────── country     [4..6]   ISO 3166-1 alpha-2 letters
//!     └────────────────── institution [0..4]   four letters
//! ```
//!
//! - The **institution code** is four letters naming the institution.
//! - The **country code** is an ISO 3166-1 alpha-2 code — see
//!   [`crate::country`].
//! - The **location code** is two `[A-Z0-9]` characters. Its second character
//!   carries a convention: `0` marks a test/training BIC and `1` a passive
//!   SWIFT participant.
//! - The **branch code**, present only in an 11-character BIC, is three
//!   `[A-Z0-9]` characters identifying a specific branch.
//!
//! A BIC has **no check digit**: [`Bic::parse`] enforces structure only —
//! one of the two permitted lengths, the per-segment character set, and a
//! country code that is a recognised ISO 3166-1 alpha-2 code.
//!
//! # References
//!
//! - ISO 9362, *Banking — Banking telecommunication messages — Business
//!   identifier code (BIC)*.

use crate::country;
use crate::errors::ValidationError;

/// A validated Business Identifier Code (ISO 9362).
///
/// A `Bic` can only be created by [`Bic::parse`] (or the explicitly unchecked
/// [`Bic::from_bytes_unchecked`]), so a value of this type is a proof that 8
/// or 11 characters form a structurally valid BIC. It stores the identifier
/// inline as `[u8; 11]` with a `len` field; the unused tail bytes of an
/// 8-character BIC are zeroed, so the derived `PartialEq`/`Eq`/`Hash` compare
/// only the significant characters. It is `Copy` and allocates nothing.
///
/// # Examples
///
/// ```
/// use regit_identifiers::Bic;
///
/// let bic = Bic::parse("DEUTDEFF500").unwrap();
/// assert_eq!(bic.institution(), "DEUT");
/// assert_eq!(bic.country_code(), "DE");
/// assert_eq!(bic.location_code(), "FF");
/// assert_eq!(bic.branch_code(), Some("500"));
/// assert_eq!(bic.as_str(), "DEUTDEFF500");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bic {
    /// The validated ASCII bytes, left-aligned; tail bytes past `len` zeroed.
    bytes: [u8; Self::MAX_LENGTH],
    /// The number of significant bytes — always either 8 or 11.
    len: u8,
}

impl Bic {
    /// The length of a BIC without a branch code.
    pub const SHORT_LENGTH: usize = 8;

    /// The length of a BIC with an explicit branch code.
    pub const MAX_LENGTH: usize = 11;

    /// Parses and validates a BIC.
    ///
    /// Validation is strict and structural — a BIC carries no check digit.
    /// In order: the input must be exactly 8 or 11 characters; the
    /// institution and country segments (characters 1–6) must each be an
    /// upper-case ASCII letter; the location and branch segments must each be
    /// an ASCII digit or upper-case letter; and the country segment must be a
    /// recognised ISO 3166-1 alpha-2 code.
    ///
    /// # Errors
    ///
    /// - [`ValidationError::Structure`] with rule `"BIC length must be 8 or
    ///   11"` if the input is neither 8 nor 11 characters.
    /// - [`ValidationError::InvalidCharacter`] if a character falls outside
    ///   the set its position allows (this also rejects lower-case input and
    ///   any non-ASCII character).
    /// - [`ValidationError::InvalidCountryCode`] if characters 5–6 are not a
    ///   recognised ISO 3166-1 alpha-2 country code.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Bic;
    /// use regit_identifiers::errors::ValidationError;
    ///
    /// assert!(Bic::parse("DEUTDEFF").is_ok());
    /// assert!(Bic::parse("DEUTDEFF500").is_ok());
    ///
    /// // A 9-character string is neither permitted length.
    /// assert_eq!(
    ///     Bic::parse("DEUTDEFF5"),
    ///     Err(ValidationError::Structure { rule: "BIC length must be 8 or 11" }),
    /// );
    /// ```
    pub fn parse(s: &str) -> Result<Self, ValidationError> {
        // A BIC is exactly 8 or 11 characters; any other length is rejected.
        let found = s.chars().count();
        if found != Self::SHORT_LENGTH && found != Self::MAX_LENGTH {
            return Err(ValidationError::Structure {
                rule: "BIC length must be 8 or 11",
            });
        }
        // Per-position character set: [0..6] are letters, [6..11] are
        // [A-Z0-9]. A non-ASCII character fails both predicates and is
        // rejected here.
        for (i, ch) in s.chars().enumerate() {
            let legal = if i < 6 {
                ch.is_ascii_uppercase()
            } else {
                ch.is_ascii_uppercase() || ch.is_ascii_digit()
            };
            if !legal {
                return Err(ValidationError::InvalidCharacter {
                    position: i + 1,
                    found: ch,
                });
            }
        }
        // Every character is ASCII, so the string is exactly `found` ASCII
        // bytes; copy them into a zeroed buffer so the tail stays zero.
        let mut bytes = [0u8; Self::MAX_LENGTH];
        let src = s.as_bytes();
        if let Some(slot) = bytes.get_mut(..found) {
            slot.copy_from_slice(src);
        }
        // Characters 5–6 must be a recognised ISO 3166-1 alpha-2 code.
        let country_code = core::str::from_utf8(&bytes[4..6]).unwrap_or("");
        if !country::is_iso_country(country_code) {
            return Err(ValidationError::InvalidCountryCode);
        }
        Ok(Self {
            bytes,
            len: u8::try_from(found).unwrap_or(0),
        })
    }

    /// Validates a BIC without constructing one.
    ///
    /// Equivalent to `Bic::parse(s).map(|_| ())`; use it when only the
    /// verdict is needed.
    ///
    /// # Errors
    ///
    /// Returns the same [`ValidationError`] variants as [`Bic::parse`].
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Bic;
    ///
    /// assert!(Bic::validate("CHASUS33").is_ok());
    /// assert!(Bic::validate("CHASUS3").is_err());
    /// ```
    pub fn validate(s: &str) -> Result<(), ValidationError> {
        Self::parse(s).map(|_| ())
    }

    /// Wraps raw bytes as a `Bic` without any validation.
    ///
    /// The caller asserts that the first `len` bytes hold the characters of a
    /// valid BIC, that `len` is 8 or 11, and that every byte from `len`
    /// onwards is zero. This exists for reconstructing a `Bic` from bytes
    /// that were validated earlier; prefer [`Bic::parse`] for any untrusted
    /// input.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Bic;
    ///
    /// // The 3 trailing zero bytes are REQUIRED, not arbitrary padding —
    /// // the derived `PartialEq`/`Eq`/`Hash` compare the full 11-byte buffer.
    /// let bic = Bic::from_bytes_unchecked(*b"DEUTDEFF\0\0\0", 8);
    /// assert_eq!(bic.as_str(), "DEUTDEFF");
    /// ```
    #[must_use]
    pub const fn from_bytes_unchecked(bytes: [u8; Self::MAX_LENGTH], len: u8) -> Self {
        Self { bytes, len }
    }

    /// Returns the number of characters in this BIC — 8 or 11.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Bic;
    ///
    /// assert_eq!(Bic::parse("DEUTDEFF").unwrap().len(), 8);
    /// assert_eq!(Bic::parse("DEUTDEFF500").unwrap().len(), 11);
    /// ```
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// Always `false` — a valid BIC is never empty (it is 8 or 11
    /// characters). Provided for API consistency with [`Bic::len`].
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Bic;
    ///
    /// assert!(!Bic::parse("DEUTDEFF").unwrap().is_empty());
    /// ```
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the BIC as a string slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Bic;
    ///
    /// assert_eq!(Bic::parse("DEUTDEFF500").unwrap().as_str(), "DEUTDEFF500");
    /// ```
    #[must_use]
    #[inline]
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(self.as_bytes()).unwrap_or("")
    }

    /// Returns the BIC as its raw ASCII bytes — 8 or 11 bytes, with no
    /// trailing zero padding.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Bic;
    ///
    /// assert_eq!(Bic::parse("DEUTDEFF").unwrap().as_bytes(), b"DEUTDEFF");
    /// ```
    #[must_use]
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.get(..self.len()).unwrap_or(&[])
    }

    /// Returns the four-character institution code, characters 1–4.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Bic;
    ///
    /// assert_eq!(Bic::parse("DEUTDEFF").unwrap().institution(), "DEUT");
    /// ```
    #[must_use]
    #[inline]
    pub fn institution(&self) -> &str {
        core::str::from_utf8(&self.bytes[0..4]).unwrap_or("")
    }

    /// Returns the two-character ISO 3166-1 country code, characters 5–6.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Bic;
    ///
    /// assert_eq!(Bic::parse("DEUTDEFF").unwrap().country_code(), "DE");
    /// ```
    #[must_use]
    #[inline]
    pub fn country_code(&self) -> &str {
        core::str::from_utf8(&self.bytes[4..6]).unwrap_or("")
    }

    /// Returns the two-character location code, characters 7–8.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Bic;
    ///
    /// assert_eq!(Bic::parse("DEUTDEFF").unwrap().location_code(), "FF");
    /// ```
    #[must_use]
    #[inline]
    pub fn location_code(&self) -> &str {
        core::str::from_utf8(&self.bytes[6..8]).unwrap_or("")
    }

    /// Returns the three-character branch code, characters 9–11, or `None`
    /// for an 8-character BIC with no branch suffix.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Bic;
    ///
    /// assert_eq!(Bic::parse("DEUTDEFF500").unwrap().branch_code(), Some("500"));
    /// assert_eq!(Bic::parse("DEUTDEFF").unwrap().branch_code(), None);
    /// ```
    #[must_use]
    #[inline]
    pub fn branch_code(&self) -> Option<&str> {
        if self.has_branch() {
            Some(core::str::from_utf8(&self.bytes[8..11]).unwrap_or(""))
        } else {
            None
        }
    }

    /// Returns `true` if this BIC carries an explicit branch code, i.e. it is
    /// 11 characters long.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Bic;
    ///
    /// assert!(Bic::parse("DEUTDEFF500").unwrap().has_branch());
    /// assert!(!Bic::parse("DEUTDEFF").unwrap().has_branch());
    /// ```
    #[must_use]
    #[inline]
    pub fn has_branch(&self) -> bool {
        self.len() == Self::MAX_LENGTH
    }

    /// Returns `true` if this is a test/training BIC, i.e. the second
    /// character of the location code (character 8) is `'0'`.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Bic;
    ///
    /// assert!(Bic::parse("DEUTDEF0").unwrap().is_test_bic());
    /// assert!(!Bic::parse("DEUTDEFF").unwrap().is_test_bic());
    /// ```
    #[must_use]
    #[inline]
    pub fn is_test_bic(&self) -> bool {
        self.bytes[7] == b'0'
    }

    /// Returns `true` if this BIC belongs to a passive SWIFT participant,
    /// i.e. the second character of the location code (character 8) is `'1'`.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Bic;
    ///
    /// assert!(Bic::parse("DEUTDEF1").unwrap().is_passive());
    /// assert!(!Bic::parse("DEUTDEFF").unwrap().is_passive());
    /// ```
    #[must_use]
    #[inline]
    pub fn is_passive(&self) -> bool {
        self.bytes[7] == b'1'
    }

    /// Returns the second character of the location code (character 8 of
    /// the BIC) — the "status character" by SWIFT convention.
    ///
    /// SWIFT attaches a convention to this character: `'0'` marks a test or
    /// training BIC ([`Bic::is_test_bic`]) and `'1'` a passive participant
    /// ([`Bic::is_passive`]). Any other value denotes a connected,
    /// non-test, non-passive participant. Use this accessor when you want
    /// the raw character; use the predicates when you only need a yes/no.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Bic;
    ///
    /// assert_eq!(Bic::parse("DEUTDEFF").unwrap().location_status(), 'F');
    /// assert_eq!(Bic::parse("DEUTDEF0").unwrap().location_status(), '0');
    /// assert_eq!(Bic::parse("DEUTDEF1").unwrap().location_status(), '1');
    /// ```
    #[must_use]
    #[inline]
    pub fn location_status(&self) -> char {
        char::from(self.bytes[7])
    }
}

impl core::fmt::Display for Bic {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl core::str::FromStr for Bic {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl AsRef<str> for Bic {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::display;
    use core::str::FromStr;

    /// Real, well-known BICs used as regression anchors.
    const GOLDEN: &[&str] = &[
        "DEUTDEFF",    // Deutsche Bank, Frankfurt (8 characters)
        "DEUTDEFF500", // Deutsche Bank, Frankfurt, branch 500 (11 characters)
        "CHASUS33",    // JPMorgan Chase Bank, New York
        "BOFAUS3N",    // Bank of America, New York
        "NDEAFIHH",    // Nordea Bank, Helsinki
    ];

    #[test]
    fn parses_golden_bics() {
        for &s in GOLDEN {
            let bic = Bic::parse(s).unwrap_or_else(|e| panic!("{s} should parse: {e}"));
            assert_eq!(bic.as_str(), s);
        }
    }

    #[test]
    fn segment_accessors_short() {
        let bic = Bic::parse("DEUTDEFF").unwrap();
        assert_eq!(bic.institution(), "DEUT");
        assert_eq!(bic.country_code(), "DE");
        assert_eq!(bic.location_code(), "FF");
        assert_eq!(bic.branch_code(), None);
        assert!(!bic.has_branch());
        assert_eq!(bic.len(), 8);
        assert_eq!(bic.as_bytes(), b"DEUTDEFF");
    }

    #[test]
    fn segment_accessors_with_branch() {
        let bic = Bic::parse("DEUTDEFF500").unwrap();
        assert_eq!(bic.institution(), "DEUT");
        assert_eq!(bic.country_code(), "DE");
        assert_eq!(bic.location_code(), "FF");
        assert_eq!(bic.branch_code(), Some("500"));
        assert!(bic.has_branch());
        assert_eq!(bic.len(), 11);
        assert_eq!(bic.as_bytes(), b"DEUTDEFF500");
    }

    #[test]
    fn length_constants() {
        assert_eq!(Bic::SHORT_LENGTH, 8);
        assert_eq!(Bic::MAX_LENGTH, 11);
    }

    #[test]
    fn test_bic_flag() {
        let bic = Bic::parse("DEUTDEF0").unwrap();
        assert!(bic.is_test_bic());
        assert!(!bic.is_passive());
    }

    #[test]
    fn passive_participant_flag() {
        let bic = Bic::parse("DEUTDEF1").unwrap();
        assert!(bic.is_passive());
        assert!(!bic.is_test_bic());
    }

    #[test]
    fn live_bic_is_neither_test_nor_passive() {
        let bic = Bic::parse("DEUTDEFF").unwrap();
        assert!(!bic.is_test_bic());
        assert!(!bic.is_passive());
    }

    #[test]
    fn location_status_returns_eighth_character() {
        assert_eq!(Bic::parse("DEUTDEFF").unwrap().location_status(), 'F');
        assert_eq!(Bic::parse("DEUTDEF0").unwrap().location_status(), '0');
        assert_eq!(Bic::parse("DEUTDEF1").unwrap().location_status(), '1');
        // Same character in an 11-character BIC.
        assert_eq!(Bic::parse("DEUTDEFF500").unwrap().location_status(), 'F');
    }

    #[test]
    fn rejects_wrong_length() {
        // Nine characters is neither 8 nor 11.
        assert_eq!(
            Bic::parse("DEUTDEFF5"),
            Err(ValidationError::Structure {
                rule: "BIC length must be 8 or 11",
            })
        );
        // Ten characters likewise.
        assert_eq!(
            Bic::parse("DEUTDEFF50"),
            Err(ValidationError::Structure {
                rule: "BIC length must be 8 or 11",
            })
        );
        // Empty input.
        assert_eq!(
            Bic::parse(""),
            Err(ValidationError::Structure {
                rule: "BIC length must be 8 or 11",
            })
        );
        // Twelve characters.
        assert_eq!(
            Bic::parse("DEUTDEFF5000"),
            Err(ValidationError::Structure {
                rule: "BIC length must be 8 or 11",
            })
        );
    }

    #[test]
    fn rejects_digit_in_institution() {
        assert!(matches!(
            Bic::parse("DEU1DEFF"),
            Err(ValidationError::InvalidCharacter { position: 4, .. })
        ));
    }

    #[test]
    fn rejects_digit_in_country() {
        // A digit anywhere in characters 5–6 must be rejected as a character
        // error, before the country lookup.
        assert!(matches!(
            Bic::parse("DEUT1EFF"),
            Err(ValidationError::InvalidCharacter { position: 5, .. })
        ));
    }

    #[test]
    fn rejects_lower_case() {
        assert!(matches!(
            Bic::parse("deutdeff"),
            Err(ValidationError::InvalidCharacter { position: 1, .. })
        ));
    }

    #[test]
    fn rejects_unknown_country_code() {
        // ZZ is two letters but not an assigned ISO 3166-1 code.
        assert_eq!(
            Bic::parse("DEUTZZFF"),
            Err(ValidationError::InvalidCountryCode)
        );
    }

    #[test]
    fn rejects_substitute_prefix_as_country() {
        // XS is an ISIN substitute prefix, not an ISO country, so it is not a
        // valid BIC country code.
        assert_eq!(
            Bic::parse("DEUTXS33"),
            Err(ValidationError::InvalidCountryCode)
        );
    }

    #[test]
    fn rejects_bad_character_in_branch() {
        assert!(matches!(
            Bic::parse("DEUTDEFF50/"),
            Err(ValidationError::InvalidCharacter { position: 11, .. })
        ));
    }

    #[test]
    fn rejects_non_ascii_without_panic() {
        // A multi-byte character must be rejected cleanly.
        assert!(Bic::parse("DEUTDEFé").is_err());
        assert!(Bic::parse("ÉEUTDEFF").is_err());
        assert!(Bic::parse("DEUTDEFF50é").is_err());
    }

    #[test]
    fn round_trips_through_str() {
        for &s in GOLDEN {
            assert_eq!(Bic::parse(s).unwrap().as_str(), s);
        }
    }

    #[test]
    fn from_str_matches_parse() {
        assert_eq!(Bic::from_str("DEUTDEFF"), Bic::parse("DEUTDEFF"));
        assert_eq!(Bic::from_str("DEUTDEFF500"), Bic::parse("DEUTDEFF500"));
        assert!(Bic::from_str("nonsense!").is_err());
    }

    #[test]
    fn display_renders_identifier() {
        assert_eq!(
            display(Bic::parse("DEUTDEFF").unwrap()).as_str(),
            "DEUTDEFF"
        );
        assert_eq!(
            display(Bic::parse("DEUTDEFF500").unwrap()).as_str(),
            "DEUTDEFF500"
        );
    }

    #[test]
    fn as_ref_str() {
        let bic = Bic::parse("CHASUS33").unwrap();
        let s: &str = bic.as_ref();
        assert_eq!(s, "CHASUS33");
    }

    #[test]
    fn validate_agrees_with_parse() {
        assert!(Bic::validate("BOFAUS3N").is_ok());
        assert!(Bic::validate("BOFAUS3").is_err());
    }

    #[test]
    fn from_bytes_unchecked_round_trip() {
        let short = Bic::from_bytes_unchecked(*b"DEUTDEFF\0\0\0", 8);
        assert_eq!(short, Bic::parse("DEUTDEFF").unwrap());
        let long = Bic::from_bytes_unchecked(*b"DEUTDEFF500", 11);
        assert_eq!(long, Bic::parse("DEUTDEFF500").unwrap());
    }

    #[test]
    fn unused_tail_bytes_are_zeroed() {
        // The tail of an 8-character BIC must be zero, so two equal 8-char
        // BICs compare equal regardless of how they were built.
        let parsed = Bic::parse("DEUTDEFF").unwrap();
        let built = Bic::from_bytes_unchecked(*b"DEUTDEFF\0\0\0", 8);
        assert_eq!(parsed, built);
    }

    #[test]
    fn is_copy_and_eq_and_hashable() {
        let a = Bic::parse("DEUTDEFF").unwrap();
        let b = a; // Copy
        assert_eq!(a, b);
        assert_ne!(a, Bic::parse("CHASUS33").unwrap());
        // An 8-char BIC and the 11-char BIC sharing its prefix differ.
        assert_ne!(a, Bic::parse("DEUTDEFF500").unwrap());
        // Usable as a map key (Eq + Hash) — checked by constructing a slice.
        let keys = [a, b];
        assert_eq!(keys[0], keys[1]);
    }
}
