// Copyright 2026 Regit.io — Nicolas Koenig
// SPDX-License-Identifier: Apache-2.0

//! CFI — Classification of Financial Instruments (ISO 10962).
//!
//! A CFI code classifies a financial instrument — what kind of thing it is,
//! rather than which specific issue. It is exactly 6 characters, all
//! upper-case letters, in three parts:
//!
//! ```text
//!   E S V U F R
//!   │ │ └──┬──┘
//!   │ │    └──── attributes  [2..6]  four characters, instrument-specific
//!   │ └───────── group       [1]     a category-dependent sub-class
//!   └─────────── category    [0]     one of 14 ISO 10962 category letters
//! ```
//!
//! - The **category** is the top-level class. It must be one of the 14
//!   ISO 10962 category letters `E C D R O F S H I J K L T M`.
//! - The **group** narrows the category into a sub-class; its meaning depends
//!   on the category.
//! - The **four attributes** further describe the instrument; an `X` in any
//!   attribute position means "not applicable / not known".
//!
//! A CFI carries **no check digit**, so any 6 upper-case letters whose first
//! character is a valid category letter form a structurally valid CFI.
//!
//! # Scope of validation
//!
//! [`Cfi::parse`] validates **structure and category only**: the exact
//! length, the all-`[A-Z]` character set, and that character 1 is a
//! recognised category letter. It deliberately does **not** check the group
//! character or the four attribute characters against the per-category
//! ISO 10962 tables. Those tables are large, category-specific, and revised
//! with each edition of the standard; validating against an embedded snapshot
//! would silently reject instruments classified under a newer revision. The
//! group and attributes are therefore exposed verbatim through accessors but
//! left semantically unvalidated.
//!
//! # References
//!
//! - ISO 10962, *Securities and related financial instruments —
//!   Classification of financial instruments (CFI) code*.

use crate::errors::ValidationError;

/// A validated Classification of Financial Instruments code (ISO 10962).
///
/// A `Cfi` can only be created by [`Cfi::parse`] (or the explicitly unchecked
/// [`Cfi::from_bytes_unchecked`]), so a value of this type is a proof that
/// the 6 characters are all upper-case letters and that the first is a
/// recognised ISO 10962 category letter. It stores the code inline as
/// `[u8; 6]`, is `Copy`, and allocates nothing.
///
/// # Examples
///
/// ```
/// use regit_identifiers::Cfi;
///
/// let cfi = Cfi::parse("ESVUFR").unwrap();
/// assert_eq!(cfi.category(), 'E');
/// assert_eq!(cfi.category_name(), "Equities");
/// assert_eq!(cfi.group(), 'S');
/// assert_eq!(cfi.attributes(), "VUFR");
/// assert_eq!(cfi.as_str(), "ESVUFR");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cfi {
    /// The 6 validated ASCII bytes of the code.
    bytes: [u8; Self::LENGTH],
}

impl Cfi {
    /// The number of characters in a CFI code.
    pub const LENGTH: usize = 6;

    /// Parses and validates a CFI code.
    ///
    /// Validation is strict and, in order: the input must be exactly 6
    /// characters; every character must be an ASCII upper-case letter; and
    /// the first character must be one of the 14 ISO 10962 category letters
    /// `E C D R O F S H I J K L T M`. A CFI has no check digit, so any
    /// 6-letter string satisfying those rules parses.
    ///
    /// The group character and the four attribute characters are **not**
    /// validated against the per-category ISO 10962 tables — see the module
    /// documentation for why.
    ///
    /// # Errors
    ///
    /// - [`ValidationError::WrongLength`] if the input is not 6 characters.
    /// - [`ValidationError::InvalidCharacter`] if a character is not an ASCII
    ///   upper-case letter (this also rejects digits, lower-case input, and
    ///   any non-ASCII character).
    /// - [`ValidationError::Structure`] if the first character is not a
    ///   recognised ISO 10962 category letter.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Cfi;
    /// use regit_identifiers::errors::ValidationError;
    ///
    /// assert!(Cfi::parse("ESVUFR").is_ok());
    ///
    /// // `Q` is not one of the 14 ISO 10962 category letters.
    /// assert_eq!(
    ///     Cfi::parse("QSVUFR"),
    ///     Err(ValidationError::Structure {
    ///         rule: "CFI category must be one of E C D R O F S H I J K L T M",
    ///     }),
    /// );
    /// ```
    pub fn parse(s: &str) -> Result<Self, ValidationError> {
        // A CFI is exactly 6 characters.
        let found = s.chars().count();
        if found != Self::LENGTH {
            return Err(ValidationError::WrongLength {
                expected: Self::LENGTH,
                found,
            });
        }
        // Every character must be an ASCII upper-case letter. A non-ASCII
        // character fails the predicate and is rejected here.
        for (i, ch) in s.chars().enumerate() {
            if !ch.is_ascii_uppercase() {
                return Err(ValidationError::InvalidCharacter {
                    position: i + 1,
                    found: ch,
                });
            }
        }
        // Every character is ASCII, so the string is exactly 6 ASCII bytes.
        let mut bytes = [0u8; Self::LENGTH];
        bytes.copy_from_slice(s.as_bytes());

        // The first character must be a recognised ISO 10962 category letter.
        if category_name_of(bytes[0]).is_none() {
            return Err(ValidationError::Structure {
                rule: "CFI category must be one of E C D R O F S H I J K L T M",
            });
        }
        Ok(Self { bytes })
    }

    /// Validates a CFI code without constructing one.
    ///
    /// Equivalent to `Cfi::parse(s).map(|_| ())`; use it when only the
    /// verdict is needed.
    ///
    /// # Errors
    ///
    /// Returns the same [`ValidationError`] variants as [`Cfi::parse`].
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Cfi;
    ///
    /// assert!(Cfi::validate("ESVUFR").is_ok());
    /// assert!(Cfi::validate("QSVUFR").is_err());
    /// ```
    pub fn validate(s: &str) -> Result<(), ValidationError> {
        Self::parse(s).map(|_| ())
    }

    /// Wraps 6 raw bytes as a `Cfi` without any validation.
    ///
    /// The caller asserts that `bytes` holds the 6 ASCII characters of a
    /// valid CFI. This exists for reconstructing a `Cfi` from bytes that were
    /// validated earlier; prefer [`Cfi::parse`] for any untrusted input.
    ///
    /// Bypassing validation has a consequence for [`Cfi::category_name`]: if
    /// `bytes[0]` is not one of the 14 ISO 10962 category letters, the
    /// accessor returns the empty string `""` (the safe fallback).
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Cfi;
    ///
    /// let cfi = Cfi::from_bytes_unchecked(*b"ESVUFR");
    /// assert_eq!(cfi.as_str(), "ESVUFR");
    /// ```
    #[must_use]
    pub const fn from_bytes_unchecked(bytes: [u8; Self::LENGTH]) -> Self {
        Self { bytes }
    }

    /// Returns the CFI code as a string slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Cfi;
    ///
    /// assert_eq!(Cfi::parse("ESVUFR").unwrap().as_str(), "ESVUFR");
    /// ```
    #[must_use]
    #[inline]
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes).unwrap_or("")
    }

    /// Returns the CFI code as its 6 raw ASCII bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Cfi;
    ///
    /// assert_eq!(Cfi::parse("ESVUFR").unwrap().as_bytes(), b"ESVUFR");
    /// ```
    #[must_use]
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the category letter, character 1.
    ///
    /// This is always one of the 14 ISO 10962 category letters.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Cfi;
    ///
    /// assert_eq!(Cfi::parse("ESVUFR").unwrap().category(), 'E');
    /// ```
    #[must_use]
    #[inline]
    pub fn category(&self) -> char {
        char::from(self.bytes[0])
    }

    /// Returns the ISO 10962 name of the category, character 1.
    ///
    /// The returned name is one of the 14 fixed category descriptions; it is
    /// never empty for a value produced by [`Cfi::parse`].
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Cfi;
    ///
    /// assert_eq!(Cfi::parse("ESVUFR").unwrap().category_name(), "Equities");
    /// assert_eq!(Cfi::parse("DBFUGR").unwrap().category_name(), "Debt instruments");
    /// ```
    #[must_use]
    #[inline]
    pub fn category_name(&self) -> &'static str {
        category_name_of(self.bytes[0]).unwrap_or("")
    }

    /// Returns the group letter, character 2.
    ///
    /// The group narrows the category into a sub-class. Its meaning is
    /// category-dependent and is **not** validated by [`Cfi::parse`].
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Cfi;
    ///
    /// assert_eq!(Cfi::parse("ESVUFR").unwrap().group(), 'S');
    /// ```
    #[must_use]
    #[inline]
    pub fn group(&self) -> char {
        char::from(self.bytes[1])
    }

    /// Returns the four attribute characters, characters 3–6.
    ///
    /// The attributes further describe the instrument; an `X` in a position
    /// means "not applicable". Their meaning is category-dependent and is
    /// **not** validated by [`Cfi::parse`].
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Cfi;
    ///
    /// assert_eq!(Cfi::parse("ESVUFR").unwrap().attributes(), "VUFR");
    /// ```
    #[must_use]
    #[inline]
    pub fn attributes(&self) -> &str {
        core::str::from_utf8(&self.bytes[2..6]).unwrap_or("")
    }
}

/// Maps an ISO 10962 category letter to its English name.
///
/// Returns `None` if `b` is not one of the 14 recognised category letters,
/// which is exactly how [`Cfi::parse`] decides whether character 1 is valid.
fn category_name_of(b: u8) -> Option<&'static str> {
    match b {
        b'E' => Some("Equities"),
        b'C' => Some("Collective investment vehicles"),
        b'D' => Some("Debt instruments"),
        b'R' => Some("Entitlements (rights)"),
        b'O' => Some("Listed options"),
        b'F' => Some("Futures"),
        b'S' => Some("Swaps"),
        b'H' => Some("Non-listed and complex listed options"),
        b'I' => Some("Spot"),
        b'J' => Some("Forwards"),
        b'K' => Some("Strategies"),
        b'L' => Some("Financing"),
        b'T' => Some("Referential instruments"),
        b'M' => Some("Others (miscellaneous)"),
        _ => None,
    }
}

impl core::fmt::Display for Cfi {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl core::str::FromStr for Cfi {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl AsRef<str> for Cfi {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::display;
    use core::str::FromStr;

    /// Well-formed CFI codes used as regression anchors.
    const GOLDEN: &[&str] = &[
        "ESVUFR", // Equities
        "DBFUGR", // Debt instruments
        "OCASPS", // Listed options
        "CIOIES", // Collective investment vehicles
    ];

    /// One representative CFI for every ISO 10962 category letter, paired
    /// with the expected category name.
    const CATEGORY_VECTORS: &[(&str, char, &str)] = &[
        ("EXXXXX", 'E', "Equities"),
        ("CXXXXX", 'C', "Collective investment vehicles"),
        ("DXXXXX", 'D', "Debt instruments"),
        ("RXXXXX", 'R', "Entitlements (rights)"),
        ("OXXXXX", 'O', "Listed options"),
        ("FXXXXX", 'F', "Futures"),
        ("SXXXXX", 'S', "Swaps"),
        ("HXXXXX", 'H', "Non-listed and complex listed options"),
        ("IXXXXX", 'I', "Spot"),
        ("JXXXXX", 'J', "Forwards"),
        ("KXXXXX", 'K', "Strategies"),
        ("LXXXXX", 'L', "Financing"),
        ("TXXXXX", 'T', "Referential instruments"),
        ("MXXXXX", 'M', "Others (miscellaneous)"),
    ];

    #[test]
    fn parses_golden_cfis() {
        for &s in GOLDEN {
            let cfi = Cfi::parse(s).unwrap_or_else(|e| panic!("{s} should parse: {e}"));
            assert_eq!(cfi.as_str(), s);
        }
    }

    #[test]
    fn segment_accessors() {
        let cfi = Cfi::parse("ESVUFR").unwrap();
        assert_eq!(cfi.category(), 'E');
        assert_eq!(cfi.category_name(), "Equities");
        assert_eq!(cfi.group(), 'S');
        assert_eq!(cfi.attributes(), "VUFR");
        assert_eq!(cfi.as_bytes(), b"ESVUFR");
        assert_eq!(Cfi::LENGTH, 6);
    }

    #[test]
    fn accepts_all_category_letters() {
        for &(s, letter, name) in CATEGORY_VECTORS {
            let cfi = Cfi::parse(s).unwrap_or_else(|e| panic!("{s} should parse: {e}"));
            assert_eq!(cfi.category(), letter);
            assert_eq!(cfi.category_name(), name);
        }
    }

    #[test]
    fn accepts_any_group_and_attributes() {
        // A CFI has no check digit and group/attributes are unvalidated, so
        // any 6-letter code with a valid category letter parses.
        let cfi = Cfi::parse("EZZZZZ").unwrap();
        assert_eq!(cfi.group(), 'Z');
        assert_eq!(cfi.attributes(), "ZZZZ");
    }

    #[test]
    fn rejects_wrong_length() {
        assert_eq!(
            Cfi::parse("ESVUF"),
            Err(ValidationError::WrongLength {
                expected: 6,
                found: 5,
            })
        );
        assert_eq!(
            Cfi::parse("ESVUFRR"),
            Err(ValidationError::WrongLength {
                expected: 6,
                found: 7,
            })
        );
        assert_eq!(
            Cfi::parse(""),
            Err(ValidationError::WrongLength {
                expected: 6,
                found: 0,
            })
        );
    }

    #[test]
    fn rejects_digit() {
        assert!(matches!(
            Cfi::parse("ESVUF1"),
            Err(ValidationError::InvalidCharacter { position: 6, .. })
        ));
    }

    #[test]
    fn rejects_lower_case() {
        assert!(matches!(
            Cfi::parse("esvufr"),
            Err(ValidationError::InvalidCharacter { position: 1, .. })
        ));
    }

    #[test]
    fn rejects_unknown_category() {
        assert_eq!(
            Cfi::parse("QSVUFR"),
            Err(ValidationError::Structure {
                rule: "CFI category must be one of E C D R O F S H I J K L T M",
            })
        );
    }

    #[test]
    fn rejects_non_ascii_without_panic() {
        // A multi-byte character must be rejected cleanly.
        assert!(Cfi::parse("ESVUFÉ").is_err());
        assert!(Cfi::parse("ÉSVUFR").is_err());
    }

    #[test]
    fn round_trips_through_str() {
        for &s in GOLDEN {
            assert_eq!(Cfi::parse(s).unwrap().as_str(), s);
        }
    }

    #[test]
    fn from_str_matches_parse() {
        assert_eq!(Cfi::from_str("ESVUFR"), Cfi::parse("ESVUFR"));
        assert!(Cfi::from_str("nonsense").is_err());
    }

    #[test]
    fn display_renders_identifier() {
        let cfi = Cfi::parse("ESVUFR").unwrap();
        assert_eq!(display(cfi).as_str(), "ESVUFR");
    }

    #[test]
    fn as_ref_str() {
        let cfi = Cfi::parse("ESVUFR").unwrap();
        let s: &str = cfi.as_ref();
        assert_eq!(s, "ESVUFR");
    }

    #[test]
    fn from_bytes_unchecked_round_trip() {
        let cfi = Cfi::from_bytes_unchecked(*b"ESVUFR");
        assert_eq!(cfi, Cfi::parse("ESVUFR").unwrap());
    }

    #[test]
    fn validate_matches_parse() {
        assert!(Cfi::validate("ESVUFR").is_ok());
        assert!(Cfi::validate("QSVUFR").is_err());
        assert!(Cfi::validate("ESVUF").is_err());
    }

    #[test]
    fn is_copy_and_eq_and_hashable() {
        let a = Cfi::parse("ESVUFR").unwrap();
        let b = a; // Copy
        assert_eq!(a, b);
        assert_ne!(a, Cfi::parse("DBFUGR").unwrap());
        // Usable as a map key (Eq + Hash) — checked by constructing a slice.
        let keys = [a, b];
        assert_eq!(keys[0], keys[1]);
    }
}
