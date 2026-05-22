// Copyright 2026 Regit.io — Nicolas Koenig
// SPDX-License-Identifier: Apache-2.0

//! Typed error enums for identifier validation and conversion.
//!
//! All failure paths return a typed `Result` — no `panic!()`, no `unwrap()`,
//! no string errors. Each variant carries enough context for the caller to
//! report precisely what was wrong and where.
//!
//! Two enums separate the two failure domains:
//!
//! - [`ValidationError`] — a string is not a well-formed identifier: wrong
//!   length, wrong charset, a failed check digit, or a structural rule.
//! - [`ConversionError`] — a structurally valid identifier cannot be mapped
//!   to the requested target identifier.
//!
//! Both implement [`core::fmt::Display`] and [`core::error::Error`], so they
//! compose with `?` and with `dyn Error` even under `#![no_std]`.
//!
//! # References
//!
//! - ISO 6166 (ISIN), ISO 7064 (check characters), ISO 9362 (BIC),
//!   ISO 10383 (MIC), ISO 10962 (CFI), ISO 17442 (LEI) — the governing
//!   standards whose rules these errors report.

use core::fmt;

// ─── Validation errors ───────────────────────────────────────────────────────

/// Error returned when a string is not a well-formed securities identifier.
///
/// Every identifier type has a strict grammar: an exact length (or a small
/// set of lengths), a per-segment character set, and — for most — a check
/// digit. A `parse` or `validate` call returns one of these variants the
/// moment an input violates that grammar.
///
/// # Examples
///
/// ```
/// use regit_identifiers::errors::ValidationError;
///
/// let err = ValidationError::WrongLength { expected: 12, found: 11 };
/// assert_eq!(err, ValidationError::WrongLength { expected: 12, found: 11 });
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationError {
    /// The input string was empty.
    Empty,
    /// The input has the wrong length for this identifier.
    WrongLength {
        /// The length the identifier requires.
        expected: usize,
        /// The length that was supplied.
        found: usize,
    },
    /// The character at 1-based `position` is not allowed there.
    InvalidCharacter {
        /// The 1-based index of the offending character.
        position: usize,
        /// The offending character.
        found: char,
    },
    /// The recomputed check digit did not match the supplied one.
    ///
    /// For multi-digit schemes (LEI) this reports the first differing digit.
    BadCheckDigit {
        /// The check digit the algorithm computed.
        expected: char,
        /// The check digit that was supplied.
        found: char,
    },
    /// The country-code segment is not a recognised code.
    InvalidCountryCode,
    /// A structural rule of the identifier was violated; `rule` names it.
    Structure {
        /// A short, human-readable description of the violated rule.
        rule: &'static str,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "input string is empty"),
            Self::WrongLength { expected, found } => {
                write!(f, "wrong length: expected {expected}, found {found}")
            }
            Self::InvalidCharacter { position, found } => {
                write!(f, "invalid character '{found}' at position {position}")
            }
            Self::BadCheckDigit { expected, found } => {
                write!(
                    f,
                    "check digit mismatch: expected '{expected}', found '{found}'"
                )
            }
            Self::InvalidCountryCode => write!(f, "unrecognised country code"),
            Self::Structure { rule } => write!(f, "structural rule violated: {rule}"),
        }
    }
}

impl core::error::Error for ValidationError {}

// ─── Conversion errors ───────────────────────────────────────────────────────

/// Error returned when one identifier cannot be converted into another.
///
/// A conversion can fail because the source identifier's country has no
/// defined target (e.g. extracting a CUSIP from a non-US/CA ISIN), because
/// the converted value is not itself a valid identifier, or because a
/// [`ValidationError`] surfaced while building the target.
///
/// # Examples
///
/// ```
/// use regit_identifiers::errors::{ConversionError, ValidationError};
///
/// // A validation error converts into a ConversionError with `?`.
/// let err: ConversionError = ValidationError::Empty.into();
/// assert!(matches!(err, ConversionError::Validation(ValidationError::Empty)));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionError {
    /// The source identifier's country has no defined target for this
    /// conversion (e.g. extracting a CUSIP from a non-US/CA ISIN).
    UnsupportedCountry,
    /// The conversion produced a value that is not itself valid; `reason`
    /// names the problem.
    NotConvertible {
        /// A short, human-readable description of why the value is invalid.
        reason: &'static str,
    },
    /// A validation error surfaced while building the converted identifier.
    Validation(ValidationError),
}

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedCountry => {
                write!(
                    f,
                    "source country has no defined target for this conversion"
                )
            }
            Self::NotConvertible { reason } => {
                write!(f, "value is not convertible: {reason}")
            }
            Self::Validation(e) => write!(f, "converted identifier is invalid: {e}"),
        }
    }
}

impl core::error::Error for ConversionError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Validation(e) => Some(e),
            _ => None,
        }
    }
}

impl From<ValidationError> for ConversionError {
    fn from(e: ValidationError) -> Self {
        Self::Validation(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{debug, display};

    #[test]
    fn validation_error_display_empty() {
        assert_eq!(
            display(ValidationError::Empty).as_str(),
            "input string is empty"
        );
    }

    #[test]
    fn validation_error_display_wrong_length() {
        let err = ValidationError::WrongLength {
            expected: 12,
            found: 11,
        };
        assert_eq!(display(err).as_str(), "wrong length: expected 12, found 11");
    }

    #[test]
    fn validation_error_display_invalid_character() {
        let err = ValidationError::InvalidCharacter {
            position: 3,
            found: '/',
        };
        assert_eq!(display(err).as_str(), "invalid character '/' at position 3");
    }

    #[test]
    fn validation_error_display_bad_check_digit() {
        let err = ValidationError::BadCheckDigit {
            expected: '5',
            found: '4',
        };
        assert_eq!(
            display(err).as_str(),
            "check digit mismatch: expected '5', found '4'"
        );
    }

    #[test]
    fn validation_error_display_country_and_structure() {
        assert_eq!(
            display(ValidationError::InvalidCountryCode).as_str(),
            "unrecognised country code"
        );
        assert_eq!(
            display(ValidationError::Structure {
                rule: "BIC length must be 8 or 11",
            })
            .as_str(),
            "structural rule violated: BIC length must be 8 or 11"
        );
    }

    #[test]
    fn validation_error_display_has_no_trailing_period() {
        for err in [
            ValidationError::Empty,
            ValidationError::WrongLength {
                expected: 1,
                found: 2,
            },
            ValidationError::InvalidCharacter {
                position: 1,
                found: 'x',
            },
            ValidationError::BadCheckDigit {
                expected: '0',
                found: '1',
            },
            ValidationError::InvalidCountryCode,
            ValidationError::Structure { rule: "r" },
        ] {
            assert!(!display(err).as_str().ends_with('.'));
        }
    }

    #[test]
    fn validation_error_is_error_trait() {
        let err: &dyn core::error::Error = &ValidationError::Empty;
        assert!(err.source().is_none());
    }

    #[test]
    fn validation_error_copy_eq() {
        let err = ValidationError::InvalidCountryCode;
        let copy = err;
        assert_eq!(err, copy);
    }

    #[test]
    fn conversion_error_display() {
        assert_eq!(
            display(ConversionError::UnsupportedCountry).as_str(),
            "source country has no defined target for this conversion"
        );
        assert!(
            display(ConversionError::NotConvertible {
                reason: "leading 00 missing",
            })
            .as_str()
            .contains("leading 00 missing")
        );
        assert!(
            display(ConversionError::Validation(ValidationError::Empty))
                .as_str()
                .contains("empty")
        );
    }

    #[test]
    fn conversion_error_from_validation_and_source() {
        let ve = ValidationError::WrongLength {
            expected: 9,
            found: 8,
        };
        let ce: ConversionError = ve.into();
        assert!(matches!(ce, ConversionError::Validation(_)));
        let dyn_err: &dyn core::error::Error = &ce;
        assert!(dyn_err.source().is_some());

        let no_src: &dyn core::error::Error = &ConversionError::UnsupportedCountry;
        assert!(no_src.source().is_none());
    }

    #[test]
    fn errors_debug() {
        assert!(debug(ValidationError::Empty).as_str().contains("Empty"));
        assert!(
            debug(ConversionError::UnsupportedCountry)
                .as_str()
                .contains("UnsupportedCountry")
        );
    }
}
