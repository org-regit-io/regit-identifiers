// Copyright 2026 Regit.io — Nicolas Koenig
// SPDX-License-Identifier: Apache-2.0

//! MIC — Market Identifier Code (ISO 10383).
//!
//! A MIC names a market — an exchange, a multilateral trading facility, or
//! another trading venue — rather than a security. It is exactly 4
//! characters with no internal segmentation:
//!
//! ```text
//!   X N A S
//!   │ └─┴─┘
//!   │   │
//!   │   └─── market suffix  [1..4]   three characters [A-Z0-9]
//!   └─────── leading letter [0]      one upper-case letter [A-Z]
//! ```
//!
//! - The **leading character** is always an upper-case ASCII letter.
//! - The remaining **three characters** are each an upper-case letter or a
//!   digit.
//! - A MIC carries **no check digit**: there is nothing to recompute. The
//!   four characters are either well-formed or they are not.
//!
//! ISO 10383 also distinguishes an *operating* MIC, which identifies a market
//! operator, from a *segment* MIC, which names a sub-market and references its
//! operating MIC.
//!
//! Structural validity is necessary but not sufficient: `ZZZZ` is well-formed
//! yet identifies no real market. True validity is membership in the published
//! ISO 10383 registry. With the default `mic-registry` feature enabled this
//! crate embeds a snapshot of that registry; [`Mic::lookup`],
//! [`Mic::is_registered`], and [`Mic::parse_registered`] consult it, and the
//! [`MicEntry`] / [`MicStatus`] types are re-exported here.
//!
//! [`Mic::parse`] enforces the structural rules; [`Mic::parse_registered`]
//! additionally requires that the code be present in the embedded registry.
//!
//! # References
//!
//! - ISO 10383, *Securities and related financial instruments — Codes for
//!   exchanges and market identification (MIC)*.

use crate::errors::ValidationError;

#[cfg(feature = "mic-registry")]
pub use crate::mic_registry::{MicEntry, MicStatus};

/// A validated Market Identifier Code (ISO 10383).
///
/// A `Mic` can only be created by [`Mic::parse`] (or the explicitly unchecked
/// [`Mic::from_bytes_unchecked`]), so a value of this type is a proof that the
/// four characters are structurally a valid MIC: a leading upper-case letter
/// followed by three upper-case alphanumeric characters. It stores the
/// identifier inline as `[u8; 4]`, is `Copy`, and allocates nothing.
///
/// Structural validity does not imply the market exists; use
/// [`Mic::is_registered`] or [`Mic::parse_registered`] to additionally check
/// the embedded ISO 10383 registry.
///
/// # Examples
///
/// ```
/// use regit_identifiers::Mic;
///
/// let mic = Mic::parse("XNAS").unwrap();
/// assert_eq!(mic.as_str(), "XNAS");
/// assert_eq!(mic.suffix(), "NAS");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Mic {
    /// The 4 validated ASCII bytes of the identifier.
    bytes: [u8; Self::LENGTH],
}

impl Mic {
    /// The number of characters in a MIC.
    pub const LENGTH: usize = 4;

    /// Parses and validates a MIC.
    ///
    /// Validation is strict and, in order: the input must be exactly 4
    /// characters; the first character must be an ASCII upper-case letter; and
    /// each of the remaining three characters must be an ASCII digit or
    /// upper-case letter. A MIC has no check digit, so there is nothing
    /// further to verify.
    ///
    /// This checks structure only — it does not consult the ISO 10383
    /// registry. Use [`Mic::parse_registered`] to additionally require that
    /// the code names a real market.
    ///
    /// # Errors
    ///
    /// - [`ValidationError::WrongLength`] if the input is not 4 characters.
    /// - [`ValidationError::InvalidCharacter`] if a character falls outside
    ///   the set its position allows (this also rejects lower-case input and
    ///   any non-ASCII character).
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Mic;
    /// use regit_identifiers::errors::ValidationError;
    ///
    /// assert!(Mic::parse("XLON").is_ok());
    ///
    /// // The leading character must be a letter, not a digit.
    /// assert_eq!(
    ///     Mic::parse("1NAS"),
    ///     Err(ValidationError::InvalidCharacter { position: 1, found: '1' }),
    /// );
    /// ```
    pub fn parse(s: &str) -> Result<Self, ValidationError> {
        // A MIC is exactly 4 characters.
        let found = s.chars().count();
        if found != Self::LENGTH {
            return Err(ValidationError::WrongLength {
                expected: Self::LENGTH,
                found,
            });
        }
        // Per-position character set: [0] is a letter, [1..4] are [A-Z0-9].
        // A non-ASCII character fails both predicates and is rejected here.
        for (i, ch) in s.chars().enumerate() {
            let legal = if i == 0 {
                ch.is_ascii_uppercase()
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
        // Every character is ASCII, so the string is exactly 4 ASCII bytes.
        let mut bytes = [0u8; Self::LENGTH];
        bytes.copy_from_slice(s.as_bytes());
        Ok(Self { bytes })
    }

    /// Validates a MIC without constructing one.
    ///
    /// Equivalent to `Mic::parse(s).map(|_| ())`; use it when only the verdict
    /// is needed.
    ///
    /// # Errors
    ///
    /// Returns the same [`ValidationError`] variants as [`Mic::parse`].
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Mic;
    ///
    /// assert!(Mic::validate("XPAR").is_ok());
    /// assert!(Mic::validate("xpar").is_err());
    /// ```
    pub fn validate(s: &str) -> Result<(), ValidationError> {
        Self::parse(s).map(|_| ())
    }

    /// Parses a MIC and requires it to be in the ISO 10383 registry.
    ///
    /// First applies the structural validation of [`Mic::parse`], then looks
    /// the code up in the embedded ISO 10383 snapshot. A well-formed but
    /// unregistered code such as `ZZZZ` is rejected here even though
    /// [`Mic::parse`] would accept it.
    ///
    /// # Errors
    ///
    /// - The same [`ValidationError`] variants as [`Mic::parse`] for a
    ///   structurally invalid input.
    /// - [`ValidationError::Structure`] with
    ///   `rule: "MIC is not in the ISO 10383 registry"` if the code is
    ///   well-formed but absent from the embedded registry.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Mic;
    /// use regit_identifiers::errors::ValidationError;
    ///
    /// // XNYS is a real, registered market.
    /// assert!(Mic::parse_registered("XNYS").is_ok());
    ///
    /// // ZZZZ is well-formed but identifies no market.
    /// assert_eq!(
    ///     Mic::parse_registered("ZZZZ"),
    ///     Err(ValidationError::Structure {
    ///         rule: "MIC is not in the ISO 10383 registry",
    ///     }),
    /// );
    /// ```
    #[cfg(feature = "mic-registry")]
    pub fn parse_registered(s: &str) -> Result<Self, ValidationError> {
        let mic = Self::parse(s)?;
        if mic.is_registered() {
            Ok(mic)
        } else {
            Err(ValidationError::Structure {
                rule: "MIC is not in the ISO 10383 registry",
            })
        }
    }

    /// Wraps 4 raw bytes as a `Mic` without any validation.
    ///
    /// The caller asserts that `bytes` holds the 4 ASCII characters of a valid
    /// MIC. This exists for reconstructing a `Mic` from bytes that were
    /// validated earlier; prefer [`Mic::parse`] for any untrusted input.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Mic;
    ///
    /// let mic = Mic::from_bytes_unchecked(*b"XNAS");
    /// assert_eq!(mic.as_str(), "XNAS");
    /// ```
    #[must_use]
    pub const fn from_bytes_unchecked(bytes: [u8; Self::LENGTH]) -> Self {
        Self { bytes }
    }

    /// Returns the MIC as a string slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Mic;
    ///
    /// assert_eq!(Mic::parse("XNAS").unwrap().as_str(), "XNAS");
    /// ```
    #[must_use]
    #[inline]
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes).unwrap_or("")
    }

    /// Returns the MIC as its 4 raw ASCII bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Mic;
    ///
    /// assert_eq!(Mic::parse("XNAS").unwrap().as_bytes(), b"XNAS");
    /// ```
    #[must_use]
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the leading character, character 1.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Mic;
    ///
    /// assert_eq!(Mic::parse("XNAS").unwrap().prefix(), 'X');
    /// ```
    #[must_use]
    #[inline]
    pub fn prefix(&self) -> char {
        char::from(self.bytes[0])
    }

    /// Returns the three-character market suffix, characters 2–4.
    ///
    /// # Examples
    ///
    /// ```
    /// use regit_identifiers::Mic;
    ///
    /// assert_eq!(Mic::parse("XNAS").unwrap().suffix(), "NAS");
    /// ```
    #[must_use]
    #[inline]
    pub fn suffix(&self) -> &str {
        core::str::from_utf8(&self.bytes[1..4]).unwrap_or("")
    }

    /// Looks the MIC up in the embedded ISO 10383 registry.
    ///
    /// Returns the [`MicEntry`] describing the market — its operating MIC,
    /// name, country, city, and status — or `None` if the code is not in the
    /// snapshot. Delegates to [`crate::mic_registry::lookup`].
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "mic-registry")] {
    /// use regit_identifiers::Mic;
    ///
    /// let mic = Mic::parse("XNAS").unwrap();
    /// let entry = mic.lookup().expect("XNAS is registered");
    /// assert_eq!(entry.mic, "XNAS");
    ///
    /// // A well-formed but unregistered code has no entry.
    /// assert!(Mic::parse("ZZZZ").unwrap().lookup().is_none());
    /// # }
    /// ```
    #[cfg(feature = "mic-registry")]
    #[must_use]
    #[inline]
    pub fn lookup(&self) -> Option<&'static MicEntry> {
        crate::mic_registry::lookup(self.as_str())
    }

    /// Returns `true` if the MIC is present in the embedded ISO 10383
    /// registry.
    ///
    /// Equivalent to `self.lookup().is_some()`.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "mic-registry")] {
    /// use regit_identifiers::Mic;
    ///
    /// assert!(Mic::parse("XLON").unwrap().is_registered());
    /// assert!(!Mic::parse("ZZZZ").unwrap().is_registered());
    /// # }
    /// ```
    #[cfg(feature = "mic-registry")]
    #[must_use]
    #[inline]
    pub fn is_registered(&self) -> bool {
        self.lookup().is_some()
    }
}

impl core::fmt::Display for Mic {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl core::str::FromStr for Mic {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl AsRef<str> for Mic {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::display;
    use core::str::FromStr;

    /// Real, registered MICs used as regression anchors.
    const GOLDEN: &[&str] = &[
        "XNAS", // Nasdaq, New York
        "XLON", // London Stock Exchange
        "XPAR", // Euronext Paris
        "XNYS", // New York Stock Exchange
    ];

    /// A structurally valid MIC that is not in the ISO 10383 registry.
    const UNREGISTERED: &str = "ZZZZ";

    #[test]
    fn parses_golden_mics() {
        for &s in GOLDEN {
            let mic = Mic::parse(s).unwrap_or_else(|e| panic!("{s} should parse: {e}"));
            assert_eq!(mic.as_str(), s);
        }
    }

    #[test]
    fn parses_unregistered_but_well_formed() {
        let mic = Mic::parse(UNREGISTERED).unwrap();
        assert_eq!(mic.as_str(), "ZZZZ");
    }

    #[test]
    fn segment_accessors() {
        let mic = Mic::parse("XNAS").unwrap();
        assert_eq!(mic.prefix(), 'X');
        assert_eq!(mic.suffix(), "NAS");
        assert_eq!(mic.as_bytes(), b"XNAS");
        assert_eq!(Mic::LENGTH, 4);
    }

    #[test]
    fn accepts_digits_in_suffix() {
        // Positions 2-4 admit digits; only position 1 must be a letter.
        let mic = Mic::parse("A2XX").unwrap();
        assert_eq!(mic.suffix(), "2XX");
    }

    #[test]
    fn rejects_wrong_length() {
        assert_eq!(
            Mic::parse("XNA"),
            Err(ValidationError::WrongLength {
                expected: 4,
                found: 3,
            })
        );
        assert_eq!(
            Mic::parse("XNASX"),
            Err(ValidationError::WrongLength {
                expected: 4,
                found: 5,
            })
        );
        assert_eq!(
            Mic::parse(""),
            Err(ValidationError::WrongLength {
                expected: 4,
                found: 0,
            })
        );
    }

    #[test]
    fn rejects_digit_in_leading_position() {
        assert_eq!(
            Mic::parse("1NAS"),
            Err(ValidationError::InvalidCharacter {
                position: 1,
                found: '1',
            })
        );
    }

    #[test]
    fn rejects_lower_case() {
        assert!(matches!(
            Mic::parse("xnas"),
            Err(ValidationError::InvalidCharacter { position: 1, .. })
        ));
        assert!(matches!(
            Mic::parse("Xnas"),
            Err(ValidationError::InvalidCharacter { position: 2, .. })
        ));
    }

    #[test]
    fn rejects_punctuation_in_suffix() {
        assert!(matches!(
            Mic::parse("XN-S"),
            Err(ValidationError::InvalidCharacter { position: 3, .. })
        ));
    }

    #[test]
    fn rejects_non_ascii_without_panic() {
        // A multi-byte character must be rejected cleanly.
        assert!(Mic::parse("XNAé").is_err());
        assert!(Mic::parse("ÉNAS").is_err());
    }

    #[test]
    fn validate_agrees_with_parse() {
        assert!(Mic::validate("XPAR").is_ok());
        assert!(Mic::validate("xpar").is_err());
    }

    #[test]
    fn round_trips_through_str() {
        for &s in GOLDEN {
            assert_eq!(Mic::parse(s).unwrap().as_str(), s);
        }
    }

    #[test]
    fn from_str_matches_parse() {
        assert_eq!(Mic::from_str("XNAS"), Mic::parse("XNAS"));
        assert!(Mic::from_str("nonsense").is_err());
    }

    #[test]
    fn display_renders_identifier() {
        let mic = Mic::parse("XNAS").unwrap();
        assert_eq!(display(mic).as_str(), "XNAS");
    }

    #[test]
    fn as_ref_str() {
        let mic = Mic::parse("XNAS").unwrap();
        let s: &str = mic.as_ref();
        assert_eq!(s, "XNAS");
    }

    #[test]
    fn from_bytes_unchecked_round_trip() {
        let mic = Mic::from_bytes_unchecked(*b"XNAS");
        assert_eq!(mic, Mic::parse("XNAS").unwrap());
    }

    #[test]
    fn is_copy_and_eq_and_hashable() {
        let a = Mic::parse("XNAS").unwrap();
        let b = a; // Copy
        assert_eq!(a, b);
        assert_ne!(a, Mic::parse("XLON").unwrap());
        // Usable as a map key (Eq + Hash) — checked by constructing a slice.
        let keys = [a, b];
        assert_eq!(keys[0], keys[1]);
    }

    #[cfg(feature = "mic-registry")]
    #[test]
    fn lookup_finds_golden_mics() {
        for &s in GOLDEN {
            let mic = Mic::parse(s).unwrap();
            let entry = mic
                .lookup()
                .unwrap_or_else(|| panic!("{s} should be registered"));
            assert_eq!(entry.mic, s);
        }
    }

    #[cfg(feature = "mic-registry")]
    #[test]
    fn lookup_misses_unregistered() {
        assert!(Mic::parse(UNREGISTERED).unwrap().lookup().is_none());
    }

    #[cfg(feature = "mic-registry")]
    #[test]
    fn is_registered_reflects_membership() {
        for &s in GOLDEN {
            assert!(Mic::parse(s).unwrap().is_registered());
        }
        assert!(!Mic::parse(UNREGISTERED).unwrap().is_registered());
    }

    #[cfg(feature = "mic-registry")]
    #[test]
    fn parse_registered_accepts_golden_mics() {
        for &s in GOLDEN {
            let mic = Mic::parse_registered(s)
                .unwrap_or_else(|e| panic!("{s} should parse as registered: {e}"));
            assert_eq!(mic.as_str(), s);
        }
    }

    #[cfg(feature = "mic-registry")]
    #[test]
    fn parse_registered_rejects_unregistered() {
        assert_eq!(
            Mic::parse_registered(UNREGISTERED),
            Err(ValidationError::Structure {
                rule: "MIC is not in the ISO 10383 registry",
            })
        );
    }

    #[cfg(feature = "mic-registry")]
    #[test]
    fn parse_registered_rejects_structural_errors_first() {
        // A structurally invalid input fails before the registry check.
        assert_eq!(
            Mic::parse_registered("XNA"),
            Err(ValidationError::WrongLength {
                expected: 4,
                found: 3,
            })
        );
        assert!(matches!(
            Mic::parse_registered("xnas"),
            Err(ValidationError::InvalidCharacter { position: 1, .. })
        ));
    }
}
